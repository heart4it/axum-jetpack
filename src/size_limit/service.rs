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
use pin_project::pin_project;
use std::convert::Infallible;
use axum::body::HttpBody;
use super::config::SizeLimitConfig;
use super::error::SizeLimitError;

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
        self.inner.poll_ready(cx).map_err(|_| unreachable!())
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

            // Buffer to hold first chunk (if any)
            let mut first_chunk = None;
            let mut total_read = 0;

            // Read the first chunk to check size immediately
            while let Some(result) = stream.next().await {
                match result {
                    Ok(chunk) => {
                        let chunk_size = chunk.len();
                        total_read += chunk_size;

                        // Check if first chunk already exceeds limit
                        if total_read > max_size {
                            // Size limit exceeded on first chunk!
                            return Ok(config.error_format.handle_error(
                                SizeLimitError::BodyTooLarge {
                                    max_size,
                                    actual_size: total_read,
                                }
                            ));
                        }

                        first_chunk = Some(chunk);
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
            let remaining_stream = Body::from_stream(stream.map(|r| r.map_err(|e| axum::Error::new(e))));
            let checking_body = SizeCheckingBody::new(remaining_stream, max_size, total_read);

            // Create final body - either with or without first chunk
            let final_body = if let Some(chunk) = first_chunk {
                // We need to create a custom body that starts with first chunk
                Body::new(ChainedBody::new(chunk, checking_body))
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

/// A body that starts with a first chunk, then continues with another body
#[pin_project]
struct ChainedBody {
    first_chunk: Option<Bytes>,
    #[pin]
    remaining: SizeCheckingBody,
}

impl ChainedBody {
    fn new(first_chunk: Bytes, remaining: SizeCheckingBody) -> Self {
        Self {
            first_chunk: Some(first_chunk),
            remaining,
        }
    }
}

impl Stream for ChainedBody {
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

impl axum::body::HttpBody for ChainedBody {
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

/// A body that checks size limit for remaining chunks
#[pin_project]
struct SizeCheckingBody {
    #[pin]
    inner: Body,
    max_size: usize,
    bytes_read: usize, // Bytes already read (including first chunk)
}

impl SizeCheckingBody {
    fn new(inner: Body, max_size: usize, bytes_read: usize) -> Self {
        Self {
            inner,
            max_size,
            bytes_read,
        }
    }
}

impl Stream for SizeCheckingBody {
    type Item = Result<Bytes, AxumError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        match this.inner.poll_frame(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some(Ok(frame))) => {
                match frame.into_data() {
                    Ok(chunk) => {
                        let chunk_size = chunk.len();

                        // Check if this chunk would exceed the limit
                        if *this.bytes_read + chunk_size > *this.max_size {
                            return Poll::Ready(Some(Err(AxumError::new(
                                SizeLimitError::BodyTooLarge {
                                    max_size: *this.max_size,
                                    actual_size: *this.bytes_read + chunk_size,
                                }
                            ))));
                        }

                        *this.bytes_read += chunk_size;
                        Poll::Ready(Some(Ok(chunk)))
                    }
                    Err(_) => Poll::Ready(None),
                }
            }
            Poll::Ready(Some(Err(e))) => {
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