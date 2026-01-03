use axum::{
    body::{Body, Bytes},
    extract::Request,
    response::Response,
    BoxError, Error as AxumError,
};
use std::sync::Arc;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use futures_util::{Stream, StreamExt};
use pin_project::{pin_project, pinned_drop};
use std::convert::Infallible;
use axum::body::HttpBody;
use super::config::SizeLimitConfig;
use super::error::SizeLimitError;

/// Maximum size for an individual chunk (16MB)
/// This prevents malicious clients from sending a single huge chunk
const MAX_INDIVIDUAL_CHUNK_SIZE: usize = 16 * 1024 * 1024;

/// Thread-safe service that checks body size based on content type
#[derive(Clone)]
pub struct SizeLimitService<S> {
    inner: S,
    config: Arc<SizeLimitConfig>,
}

impl<S> SizeLimitService<S> {
    pub fn new(inner: S, config: SizeLimitConfig) -> Self {
        Self {
            inner,
            config: Arc::new(config),
        }
    }
}

impl<S> tower::Service<Request<Body>> for SizeLimitService<S>
where
    S: tower::Service<Request<Body>, Response = Response> + Clone + Send + Sync + 'static,
    S::Error: Into<BoxError>,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // Convert inner error to Infallible instead of panicking
        match self.inner.poll_ready(cx) {
            Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
            Poll::Ready(Err(_)) => {
                // Inner service error - treat as not ready rather than panicking
                Poll::Pending
            }
            Poll::Pending => Poll::Pending,
        }
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let config = self.config.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            // Extract content type BEFORE moving request
            let content_type = request.headers()
                .get("content-type")
                .and_then(|ct| ct.to_str().ok())
                .unwrap_or("application/octet-stream");

            // Get the appropriate size limit
            let max_size = config.get_limit_for_content_type(content_type);

            // Extract the body
            let (parts, body) = request.into_parts();

            // Convert body to stream to read first chunk
            let mut stream = body.into_data_stream();

            // Buffer to hold first chunk (if any) - using Bytes to avoid double allocation
            let mut first_chunk_bytes: Option<Bytes> = None;
            let mut total_read = 0usize;

            // Read the first chunk to check size immediately
            while let Some(result) = stream.next().await {
                match result {
                    Ok(chunk) => {
                        let chunk_size = chunk.len();

                        // Check individual chunk size before any other processing
                        if chunk_size > MAX_INDIVIDUAL_CHUNK_SIZE {
                            return Ok(config.error_format.handle_error(
                                SizeLimitError::ChunkTooLarge {
                                    max_chunk_size: MAX_INDIVIDUAL_CHUNK_SIZE,
                                    actual_chunk_size: chunk_size,
                                }
                            ));
                        }

                        // Use checked_add to prevent integer overflow
                        total_read = match total_read.checked_add(chunk_size) {
                            Some(new_total) => new_total,
                            None => {
                                // Integer overflow - treat as size limit exceeded
                                return Ok(config.error_format.handle_error(
                                    SizeLimitError::SizeOverflow
                                ));
                            }
                        };

                        // Check size BEFORE storing chunk in memory
                        if total_read > max_size {
                            // Size limit exceeded on first chunk!
                            return Ok(config.error_format.handle_error(
                                SizeLimitError::BodyTooLarge {
                                    max_size,
                                    actual_size: total_read,
                                }
                            ));
                        }

                        // Now store the chunk (safe since we passed all checks)
                        first_chunk_bytes = Some(chunk);
                        break; // We only need first chunk for immediate check
                    }
                    Err(e) => {
                        // Stream error - return 400
                        return Ok(Response::builder()
                            .status(400)
                            .body(Body::from(format!("Body stream error: {}", e)))
                            .unwrap());
                    }
                }
            }

            // Create the checking body for remaining stream
            let remaining_stream = Body::from_stream(SizeLimitedStream::new(
                stream,
                max_size,
                total_read
            ));

            let checking_body = SizeCheckingBody::new(remaining_stream, max_size, total_read);

            // Avoid double buffering by directly passing the chunk
            let final_body = if let Some(chunk) = first_chunk_bytes {
                // Create a stream that yields the chunk first, then continues
                Body::from_stream(ChainedStream::new(chunk, checking_body))
            } else {
                // No first chunk, just use checking body
                Body::new(checking_body)
            };

            // Create new request
            let new_request = Request::from_parts(parts, final_body);

            // Pass to inner service
            match inner.call(new_request).await {
                Ok(response) => Ok(response),
                Err(err) => {
                    // Handler failed - could be due to size limit in remaining chunks
                    let box_error: BoxError = err.into();

                    // Check if error contains size limit message
                    let error_str = box_error.to_string();
                    if error_str.contains("Body too large") || error_str.contains("Maximum size is") {
                        // Try to extract actual size from error message
                        let actual_size = extract_actual_size_from_error(&error_str).unwrap_or(max_size + 1);

                        return Ok(config.error_format.handle_error(
                            SizeLimitError::BodyTooLarge {
                                max_size,
                                actual_size,
                            }
                        ));
                    }

                    // Some other error - return 500
                    eprintln!("Unexpected error in size limit middleware: {}", error_str);
                    Ok(Response::builder()
                        .status(500)
                        .body(Body::from("Internal server error"))
                        .unwrap())
                }
            }
        })
    }
}

/// Helper to extract actual size from error message (simplified)
fn extract_actual_size_from_error(error: &str) -> Option<usize> {
    // Look for pattern like "got 123 bytes"
    error.find("got ")
        .and_then(|pos| error[pos+4..].split_whitespace().next())
        .and_then(|s| s.parse::<usize>().ok())
}

/// A stream that yields a first chunk, then continues with another stream
#[pin_project]
struct ChainedStream {
    first_chunk: Option<Bytes>,
    #[pin]
    remaining: SizeCheckingBody,
}

impl ChainedStream {
    fn new(first_chunk: Bytes, remaining: SizeCheckingBody) -> Self {
        Self {
            first_chunk: Some(first_chunk),
            remaining,
        }
    }
}

impl Stream for ChainedStream {
    type Item = Result<Bytes, AxumError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        // First, yield the first chunk if we have it
        if let Some(chunk) = this.first_chunk.take() {
            return Poll::Ready(Some(Ok(chunk)));
        }

        // Then continue with the remaining body
        this.remaining.poll_next(cx)
    }
}

impl axum::body::HttpBody for ChainedStream {
    type Data = Bytes;
    type Error = AxumError;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<hyper::body::Frame<Self::Data>, Self::Error>>> {
        match self.as_mut().poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some(Ok(chunk))) => {
                Poll::Ready(Some(Ok(hyper::body::Frame::data(chunk))))
            }
            Poll::Ready(Some(Err(e))) => {
                Poll::Ready(Some(Err(e)))
            }
        }
    }
}

/// A stream wrapper that ensures size limits are enforced
#[pin_project]
struct SizeLimitedStream<S> {
    #[pin]
    inner: S,
    max_size: usize,
    bytes_read: usize,
    has_errored: bool, // Track if we've already returned an error
}

impl<S> SizeLimitedStream<S> {
    fn new(inner: S, max_size: usize, initial_bytes: usize) -> Self {
        Self {
            inner,
            max_size,
            bytes_read: initial_bytes,
            has_errored: false,
        }
    }
}

impl<S, E> Stream for SizeLimitedStream<S>
where
    S: Stream<Item = Result<Bytes, E>>,
    E: Into<BoxError>,
{
    type Item = Result<Bytes, AxumError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        // If we've already errored, don't poll further
        if *this.has_errored {
            return Poll::Ready(None);
        }

        match this.inner.poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some(Ok(chunk))) => {
                let chunk_size = chunk.len();

                // Check individual chunk size
                if chunk_size > MAX_INDIVIDUAL_CHUNK_SIZE {
                    *this.has_errored = true;
                    return Poll::Ready(Some(Err(AxumError::new(
                        SizeLimitError::ChunkTooLarge {
                            max_chunk_size: MAX_INDIVIDUAL_CHUNK_SIZE,
                            actual_chunk_size: chunk_size,
                        }
                    ))));
                }

                // Check cumulative size with overflow protection
                match this.bytes_read.checked_add(chunk_size) {
                    Some(new_total) => {
                        if new_total > *this.max_size {
                            *this.has_errored = true;
                            return Poll::Ready(Some(Err(AxumError::new(
                                SizeLimitError::BodyTooLarge {
                                    max_size: *this.max_size,
                                    actual_size: new_total,
                                }
                            ))));
                        }
                        *this.bytes_read = new_total;
                    }
                    None => {
                        *this.has_errored = true;
                        return Poll::Ready(Some(Err(AxumError::new(
                            SizeLimitError::SizeOverflow
                        ))));
                    }
                }

                Poll::Ready(Some(Ok(chunk)))
            }
            Poll::Ready(Some(Err(e))) => {
                *this.has_errored = true;
                Poll::Ready(Some(Err(AxumError::new(e))))
            }
        }
    }
}

/// A body that checks size limit for remaining chunks
/// Note: We use PinnedDrop because we have a manual Drop implementation
#[pin_project(PinnedDrop)]
struct SizeCheckingBody {
    #[pin]
    inner: Body,
    max_size: usize,
    bytes_read: usize, // Bytes already read (including first chunk)
    has_errored: bool, // Track if we've already returned an error
}

impl SizeCheckingBody {
    fn new(inner: Body, max_size: usize, bytes_read: usize) -> Self {
        Self {
            inner,
            max_size,
            bytes_read,
            has_errored: false,
        }
    }
}

#[pinned_drop]
impl PinnedDrop for SizeCheckingBody {
    fn drop(self: Pin<&mut Self>) {
        // This runs when the type is dropped
        // We can access fields here but they're pinned
        let this = self.project();

        // Log if body wasn't fully consumed (for debugging)
        if !*this.has_errored && *this.bytes_read < *this.max_size {
            // The body wasn't fully consumed - this is normal for early exits
            // The inner Body will handle its own cleanup
        }
    }
}

impl Stream for SizeCheckingBody {
    type Item = Result<Bytes, AxumError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        // If we've already errored, don't poll further
        if *this.has_errored {
            return Poll::Ready(None);
        }

        match this.inner.poll_frame(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some(Ok(frame))) => {
                match frame.into_data() {
                    Ok(chunk) => {
                        let chunk_size = chunk.len();

                        // Check individual chunk size
                        if chunk_size > MAX_INDIVIDUAL_CHUNK_SIZE {
                            *this.has_errored = true;
                            return Poll::Ready(Some(Err(AxumError::new(
                                SizeLimitError::ChunkTooLarge {
                                    max_chunk_size: MAX_INDIVIDUAL_CHUNK_SIZE,
                                    actual_chunk_size: chunk_size,
                                }
                            ))));
                        }

                        // Check if this chunk would exceed the limit with overflow protection
                        match this.bytes_read.checked_add(chunk_size) {
                            Some(new_total) => {
                                if new_total > *this.max_size {
                                    *this.has_errored = true;
                                    return Poll::Ready(Some(Err(AxumError::new(
                                        SizeLimitError::BodyTooLarge {
                                            max_size: *this.max_size,
                                            actual_size: new_total,
                                        }
                                    ))));
                                }
                                *this.bytes_read = new_total;
                            }
                            None => {
                                *this.has_errored = true;
                                return Poll::Ready(Some(Err(AxumError::new(
                                    SizeLimitError::SizeOverflow
                                ))));
                            }
                        }

                        Poll::Ready(Some(Ok(chunk)))
                    }
                    Err(_) => Poll::Ready(None),
                }
            }
            Poll::Ready(Some(Err(e))) => {
                *this.has_errored = true;
                Poll::Ready(Some(Err(AxumError::new(e))))
            }
        }
    }
}

impl axum::body::HttpBody for SizeCheckingBody {
    type Data = Bytes;
    type Error = AxumError;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<hyper::body::Frame<Self::Data>, Self::Error>>> {
        match self.as_mut().poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some(Ok(chunk))) => {
                Poll::Ready(Some(Ok(hyper::body::Frame::data(chunk))))
            }
            Poll::Ready(Some(Err(e))) => {
                Poll::Ready(Some(Err(e)))
            }
        }
    }
}