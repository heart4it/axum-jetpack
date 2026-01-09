//! Request size limiting middleware for Axum applications.
//!
//! This module provides middleware that enforces size limits on incoming HTTP requests
//! with configurable strategies for handling different content types.
//! It supports both buffered and streamed processing based on content type patterns.

use axum::body::to_bytes;
use axum::{
    Router,
    body::{Body, Bytes},
    extract::{Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Response},
};
use futures::StreamExt;
use std::sync::Arc;
use tokio_stream::wrappers::ReceiverStream;

use crate::size_limit::SizeLimitConfig;

/// Defines strategy for whether to buffer or stream requests based on content type.
///
/// This allows different handling strategies for different types of content:
/// - **Buffered**: Entire request body is loaded into memory before processing
///   (better for small, structured data like JSON)
/// - **Streamed**: Request body is processed in chunks as it arrives
///   (better for large files like videos or images)
///
/// Content types can be specified with exact matches or wildcards (e.g., "image/*").
#[derive(Clone, Debug)]
pub struct BufferStrategy {
    /// Content types that should be fully buffered into memory before processing.
    /// Examples: ["application/json", "text/*", "multipart/form-data"]
    pub buffered_types: Vec<String>,

    /// Content types that should be streamed (processed in chunks as they arrive).
    /// Examples: ["video/*", "image/*", "application/octet-stream"]
    pub streamed_types: Vec<String>,

    /// Default behavior for content types not explicitly listed in either list.
    /// If `true`, unlisted types will be buffered; if `false`, they will be streamed.
    pub default_is_buffered: bool,
}

impl BufferStrategy {
    /// Creates a new, empty buffer strategy.
    ///
    /// # Example
    /// ```rust
    /// use axum_jetpack::size_limit::middleware::BufferStrategy;
    ///
    /// let strategy = BufferStrategy::new();
    /// ```
    pub fn new() -> Self {
        Self {
            buffered_types: Vec::new(),
            streamed_types: Vec::new(),
            default_is_buffered: false,
        }
    }

    /// Creates a buffer strategy with sensible defaults for web applications.
    ///
    /// Default buffered types:
    /// - `application/json` - API requests
    /// - `multipart/form-data` - Form uploads
    /// - `text/*` - Text content
    /// - `application/xml` - XML data
    /// - `application/x-www-form-urlencoded` - Form data
    ///
    /// Default streamed types:
    /// - `video/*` - Video files
    /// - `image/*` - Images
    /// - `audio/*` - Audio files
    /// - `application/octet-stream` - Binary files
    ///
    /// # Example
    /// ```rust
    /// use axum_jetpack::size_limit::middleware::BufferStrategy;
    ///
    /// let strategy = BufferStrategy::with_defaults();
    /// ```
    pub fn with_defaults() -> Self {
        Self {
            buffered_types: vec![
                "application/json".to_string(),
                "multipart/form-data".to_string(),
                "text/*".to_string(),
                "application/xml".to_string(),
                "application/x-www-form-urlencoded".to_string(),
            ],
            streamed_types: vec![
                "video/*".to_string(),
                "image/*".to_string(),
                "audio/*".to_string(),
                "application/octet-stream".to_string(),
            ],
            default_is_buffered: false, // Stream by default for unknown types
        }
    }

    /// Creates a strategy that buffers ALL request bodies regardless of content type.
    ///
    /// Useful when you want to ensure all requests are fully buffered,
    /// but be cautious with memory usage for large uploads.
    ///
    /// # Example
    /// ```rust
    /// use axum_jetpack::size_limit::middleware::BufferStrategy;
    ///
    /// let strategy = BufferStrategy::all_buffered();
    /// ```
    pub fn all_buffered() -> Self {
        Self {
            buffered_types: Vec::new(),
            streamed_types: Vec::new(),
            default_is_buffered: true,
        }
    }

    /// Creates a strategy that streams ALL request bodies regardless of content type.
    ///
    /// Useful for proxy servers or when memory is constrained.
    ///
    /// # Example
    /// ```rust
    /// use axum_jetpack::size_limit::middleware::BufferStrategy;
    ///
    /// let strategy = BufferStrategy::all_streamed();
    /// ```
    pub fn all_streamed() -> Self {
        Self {
            buffered_types: Vec::new(),
            streamed_types: Vec::new(),
            default_is_buffered: false,
        }
    }

    /// Builder method to add content types that should be buffered.
    ///
    /// # Arguments
    /// * `types` - Slice of content type patterns (supports wildcards like "text/*")
    ///
    /// # Example
    /// ```rust
    /// use axum_jetpack::size_limit::middleware::BufferStrategy;
    ///
    /// let strategy = BufferStrategy::new()
    ///     .with_buffered_types(&["application/json", "text/*"]);
    /// ```
    pub fn with_buffered_types(mut self, types: &[&str]) -> Self {
        self.buffered_types
            .extend(types.iter().map(|s| s.to_string()));
        self
    }

    /// Builder method to add content types that should be streamed.
    ///
    /// # Arguments
    /// * `types` - Slice of content type patterns (supports wildcards like "video/*")
    ///
    /// # Example
    /// ```rust
    /// use axum_jetpack::size_limit::middleware::BufferStrategy;
    ///
    /// let strategy = BufferStrategy::new()
    ///     .with_streamed_types(&["video/*", "image/png"]);
    /// ```
    pub fn with_streamed_types(mut self, types: &[&str]) -> Self {
        self.streamed_types
            .extend(types.iter().map(|s| s.to_string()));
        self
    }

    /// Builder method to set the default behavior for unlisted content types.
    ///
    /// # Arguments
    /// * `is_buffered` - If `true`, unlisted types will be buffered; if `false`, streamed
    ///
    /// # Example
    /// ```rust
    /// use axum_jetpack::size_limit::middleware::BufferStrategy;
    ///
    /// // Buffer unknown types by default
    /// let strategy = BufferStrategy::new()
    ///     .with_default_buffered(true);
    /// ```
    pub fn with_default_buffered(mut self, is_buffered: bool) -> Self {
        self.default_is_buffered = is_buffered;
        self
    }

    /// Clears all buffered content type patterns.
    ///
    /// # Example
    /// ```rust
    /// use axum_jetpack::size_limit::middleware::BufferStrategy;
    ///
    /// let mut strategy = BufferStrategy::with_defaults();
    /// strategy.clear_buffered_types();
    /// ```
    pub fn clear_buffered_types(&mut self) {
        self.buffered_types.clear();
    }

    /// Clears all streamed content type patterns.
    ///
    /// # Example
    /// ```rust
    /// use axum_jetpack::size_limit::middleware::BufferStrategy;
    ///
    /// let mut strategy = BufferStrategy::with_defaults();
    /// strategy.clear_streamed_types();
    /// ```
    pub fn clear_streamed_types(&mut self) {
        self.streamed_types.clear();
    }

    /// Clears both buffered and streamed content type patterns.
    ///
    /// # Example
    /// ```rust
    /// use axum_jetpack::size_limit::middleware::BufferStrategy;
    ///
    /// let mut strategy = BufferStrategy::with_defaults();
    /// strategy.clear_all_types();
    /// ```
    pub fn clear_all_types(&mut self) {
        self.buffered_types.clear();
        self.streamed_types.clear();
    }

    /// Determines whether a given content type should be buffered or streamed.
    ///
    /// The decision logic follows this order:
    /// 1. Exact match in `buffered_types` -> buffer
    /// 2. Exact match in `streamed_types` -> stream
    /// 3. Wildcard match in `buffered_types` -> buffer
    /// 4. Wildcard match in `streamed_types` -> stream
    /// 5. Fall back to `default_is_buffered`
    ///
    /// # Arguments
    /// * `content_type` - The Content-Type header value (may include charset, e.g., "application/json; charset=utf-8")
    ///
    /// # Returns
    /// `true` if the content should be buffered, `false` if it should be streamed.
    ///
    /// # Example
    /// ```rust
    /// use axum_jetpack::size_limit::middleware::BufferStrategy;
    ///
    /// let strategy = BufferStrategy::with_defaults();
    /// assert!(strategy.should_buffer("application/json"));
    /// assert!(!strategy.should_buffer("video/mp4"));
    /// ```
    pub fn should_buffer(&self, content_type: &str) -> bool {
        // Normalize the content type: lowercase and remove charset/semantic
        let ct_lower = content_type.to_lowercase();
        let ct_trimmed = ct_lower.split(';').next().unwrap_or(&ct_lower).trim();

        // Check for exact matches first (highest priority)
        if self.buffered_types.iter().any(|t| t == ct_trimmed) {
            return true;
        }
        if self.streamed_types.iter().any(|t| t == ct_trimmed) {
            return false;
        }

        // Check for wildcard matches (e.g., "image/*" matches "image/png")
        if let Some(slash_pos) = ct_trimmed.find('/') {
            let wildcard = format!("{}/*", &ct_trimmed[..slash_pos]);

            if self.buffered_types.iter().any(|t| t == &wildcard) {
                return true;
            }
            if self.streamed_types.iter().any(|t| t == &wildcard) {
                return false;
            }
        }

        // Check for partial wildcard matches (handles already defined wildcards)
        for buffered in &self.buffered_types {
            if buffered.ends_with("/*") && ct_trimmed.starts_with(&buffered[..buffered.len() - 1]) {
                return true;
            }
        }

        for streamed in &self.streamed_types {
            if streamed.ends_with("/*") && ct_trimmed.starts_with(&streamed[..streamed.len() - 1]) {
                return false;
            }
        }

        // Fall back to default behavior
        self.default_is_buffered
    }
}

impl Default for BufferStrategy {
    /// Returns the default buffer strategy (same as `BufferStrategy::with_defaults()`).
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Configuration for the size limit middleware.
///
/// Combines size limits with buffering strategy to provide comprehensive
/// control over how different types of requests are handled.
#[derive(Clone)]
pub struct SizeLimitMiddlewareConfig {
    /// Size limits configuration per content type.
    pub size_limits: SizeLimitConfig,

    /// Strategy for deciding which content types to buffer vs. stream.
    pub buffer_strategy: BufferStrategy,
}

impl SizeLimitMiddlewareConfig {
    /// Creates a new middleware configuration with empty buffer strategy.
    ///
    /// # Arguments
    /// * `size_limits` - Configuration for size limits per content type
    ///
    /// # Example
    /// ```rust
    /// use axum_jetpack::size_limit::{SizeLimitConfig, middleware::SizeLimitMiddlewareConfig};
    ///
    /// let config = SizeLimitMiddlewareConfig::new(SizeLimitConfig::default());
    /// ```
    pub fn new(size_limits: SizeLimitConfig) -> Self {
        Self {
            size_limits,
            buffer_strategy: BufferStrategy::new(),
        }
    }

    /// Creates a middleware configuration with default buffer strategy.
    ///
    /// # Arguments
    /// * `size_limits` - Configuration for size limits per content type
    ///
    /// # Example
    /// ```rust
    /// use axum_jetpack::size_limit::{SizeLimitConfig, middleware::SizeLimitMiddlewareConfig};
    ///
    /// let config = SizeLimitMiddlewareConfig::with_default_buffer_strategy(
    ///     SizeLimitConfig::default()
    /// );
    /// ```
    pub fn with_default_buffer_strategy(size_limits: SizeLimitConfig) -> Self {
        Self {
            size_limits,
            buffer_strategy: BufferStrategy::with_defaults(),
        }
    }

    /// Builder method to set a custom buffer strategy.
    ///
    /// # Arguments
    /// * `strategy` - The buffer strategy to use
    ///
    /// # Example
    /// ```rust
    /// use axum_jetpack::size_limit::middleware::{BufferStrategy, SizeLimitMiddlewareConfig};
    ///
    /// let strategy = BufferStrategy::all_buffered();
    /// let config = SizeLimitMiddlewareConfig::default()
    ///     .with_buffer_strategy(strategy);
    /// ```
    pub fn with_buffer_strategy(mut self, strategy: BufferStrategy) -> Self {
        self.buffer_strategy = strategy;
        self
    }

    /// Builder method to add buffered content types.
    ///
    /// # Arguments
    /// * `types` - Slice of content type patterns to buffer
    ///
    /// # Example
    /// ```rust
    /// use axum_jetpack::size_limit::middleware::SizeLimitMiddlewareConfig;
    ///
    /// let config = SizeLimitMiddlewareConfig::default()
    ///     .with_buffered_types(&["application/custom+json"]);
    /// ```
    pub fn with_buffered_types(mut self, types: &[&str]) -> Self {
        self.buffer_strategy = self.buffer_strategy.with_buffered_types(types);
        self
    }

    /// Builder method to add streamed content types.
    ///
    /// # Arguments
    /// * `types` - Slice of content type patterns to stream
    ///
    /// # Example
    /// ```rust
    /// use axum_jetpack::size_limit::middleware::SizeLimitMiddlewareConfig;
    ///
    /// let config = SizeLimitMiddlewareConfig::default()
    ///     .with_streamed_types(&["model/gltf-binary"]);
    /// ```
    pub fn with_streamed_types(mut self, types: &[&str]) -> Self {
        self.buffer_strategy = self.buffer_strategy.with_streamed_types(types);
        self
    }

    /// Builder method to set default buffering behavior.
    ///
    /// # Arguments
    /// * `is_buffered` - Default behavior for unlisted content types
    ///
    /// # Example
    /// ```rust
    /// use axum_jetpack::size_limit::middleware::SizeLimitMiddlewareConfig;
    ///
    /// // Buffer unknown types by default (more conservative)
    /// let config = SizeLimitMiddlewareConfig::default()
    ///     .with_default_buffered(true);
    /// ```
    pub fn with_default_buffered(mut self, is_buffered: bool) -> Self {
        self.buffer_strategy = self.buffer_strategy.with_default_buffered(is_buffered);
        self
    }
}

impl Default for SizeLimitMiddlewareConfig {
    /// Returns a default middleware configuration with default size limits
    /// and default buffer strategy.
    fn default() -> Self {
        Self {
            size_limits: SizeLimitConfig::default(),
            buffer_strategy: BufferStrategy::with_defaults(),
        }
    }
}

/// Applies size limiting middleware to an Axum router.
///
/// This middleware:
/// 1. Inspects the Content-Type header of incoming requests
/// 2. Checks Content-Length header for quick early rejection of obviously oversized requests
/// 3. Uses the buffer strategy to decide whether to buffer or stream the request
/// 4. Enforces size limits during processing
/// 5. Returns 413 (Payload Too Large) if limits are exceeded
///
/// # Arguments
/// * `router` - The Axum router to wrap with middleware
/// * `config` - Configuration for size limits and buffering strategy
///
/// # Returns
/// A new router with size limiting middleware applied.
///
/// # Example
/// ```rust
/// use axum::{Router, routing::post};
/// use axum_jetpack::size_limit::{SizeLimitConfig, middleware::SizeLimitMiddlewareConfig, middleware::with_size_limit};
///
/// async fn upload_handler() -> &'static str {
///     "Upload received"
/// }
///
/// let router = Router::new()
///     .route("/upload", post(upload_handler));
///
/// let config = SizeLimitMiddlewareConfig::default();
/// let router = with_size_limit(router, config);
/// ```
pub fn with_size_limit(router: Router, config: SizeLimitMiddlewareConfig) -> Router {
    let config = Arc::new(config);

    router.layer(middleware::from_fn_with_state(
        config,
        |State(config): State<Arc<SizeLimitMiddlewareConfig>>, req: Request<Body>, next: Next| async move {
            // Extract and normalize Content-Type header
            let content_type = req.headers()
                .get(axum::http::header::CONTENT_TYPE)
                .and_then(|h| h.to_str().ok())
                .unwrap_or("application/octet-stream"); // Default for unknown types

            // Get size limit for this content type
            let limit = config.size_limits.get_limit_for_content_type(content_type);

            // Early rejection based on Content-Length header (if present)
            if let Some(content_length) = req.headers().get(axum::http::header::CONTENT_LENGTH)
                && let Ok(length_str) = content_length.to_str()
                    && let Ok(content_length_value) = length_str.parse::<usize>()
                        && content_length_value > limit {
                            // Request is already too large based on Content-Length header
                            return Ok((StatusCode::PAYLOAD_TOO_LARGE, "Payload too large").into_response());
                        }

            // Choose processing strategy based on content type
            if config.buffer_strategy.should_buffer(content_type) {
                buffer_with_limit(req, next, limit).await
            } else {
                stream_with_limit(req, next, limit).await
            }
        }
    ))
}

/// Applies size limiting middleware with a simplified configuration.
///
/// This is a convenience wrapper that creates a default buffer strategy
/// using the provided size limits.
///
/// # Arguments
/// * `router` - The Axum router to wrap with middleware
/// * `size_limits` - Size limits configuration
///
/// # Returns
/// A new router with size limiting middleware applied.
///
/// # Example
/// ```rust
/// use axum::{Router, routing::post};
/// use axum_jetpack::size_limit::{SizeLimitConfig, middleware::with_size_limit_simple};
///
/// async fn api_handler() -> &'static str {
///     "API endpoint"
/// }
///
/// let router = Router::new()
///     .route("/api", post(api_handler));
///
/// let limits = SizeLimitConfig::default()
///     .with_default_limit(1024 * 1024); // 1MB
///
/// let router = with_size_limit_simple(router, limits);
/// ```
pub fn with_size_limit_simple(router: Router, size_limits: SizeLimitConfig) -> Router {
    let config = SizeLimitMiddlewareConfig::new(size_limits);
    with_size_limit(router, config)
}

/// Processes a request with buffering strategy.
///
/// This function:
/// 1. Reads the entire request body into memory
/// 2. Checks if it exceeds the size limit
/// 3. If within limits, continues processing
/// 4. If exceeds limits, returns 413 (Payload Too Large)
///
/// # Arguments
/// * `req` - The HTTP request
/// * `next` - The next middleware/handler in the chain
/// * `max_size` - Maximum allowed size in bytes
///
/// # Returns
/// HTTP response or 413 error if size limit is exceeded.
async fn buffer_with_limit(
    mut req: Request<Body>,
    next: Next,
    max_size: usize,
) -> Result<Response, StatusCode> {
    use axum::response::IntoResponse;

    // Take ownership of the request body
    let body = std::mem::take(req.body_mut());

    // Read entire body into memory with size limit
    match to_bytes(body, max_size).await {
        Ok(bytes) => {
            // Double-check size (to_bytes may read exactly max_size without error)
            if bytes.len() > max_size {
                return Ok((StatusCode::PAYLOAD_TOO_LARGE, "Payload too large").into_response());
            }

            // Replace request body with buffered bytes
            *req.body_mut() = Body::from(bytes);

            // Continue to next middleware/handler
            Ok(next.run(req).await)
        }
        Err(_) => {
            // Body exceeded limit or other read error
            Ok((StatusCode::PAYLOAD_TOO_LARGE, "Payload too large").into_response())
        }
    }
}

/// Processes a request with streaming strategy.
///
/// This function:
/// 1. Sets up a streaming pipeline with size monitoring
/// 2. Processes chunks as they arrive
/// 3. Tracks total size and stops processing if limit is exceeded
/// 4. Returns 413 if limit is exceeded during streaming
///
/// # Arguments
/// * `req` - The HTTP request
/// * `next` - The next middleware/handler in the chain
/// * `max_size` - Maximum allowed size in bytes
///
/// # Returns
/// HTTP response or 413 error if size limit is exceeded during streaming.
async fn stream_with_limit(
    req: Request<Body>,
    next: Next,
    max_size: usize,
) -> Result<Response, StatusCode> {
    use axum::response::IntoResponse;

    // Create a channel for streaming the body with backpressure
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Bytes, axum::Error>>(32);
    let (parts, body) = req.into_parts();

    // Shared flag to indicate if size limit was exceeded
    let limit_exceeded = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let limit_exceeded_clone = limit_exceeded.clone();

    // Channel to communicate if we should call the next handler
    let (handler_tx, handler_rx) = tokio::sync::oneshot::channel::<bool>();

    // Spawn a task to read and forward the stream with size checking
    tokio::spawn(async move {
        let mut stream = body.into_data_stream();
        let mut total_size = 0usize;
        let mut should_call_handler = true;

        // Process stream chunks
        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    total_size += chunk.len();

                    // Check if we've exceeded the limit
                    if total_size > max_size {
                        limit_exceeded_clone.store(true, std::sync::atomic::Ordering::SeqCst);
                        should_call_handler = false;
                        break;
                    }

                    // Forward chunk to the receiver
                    if tx.send(Ok(chunk)).await.is_err() {
                        // Receiver dropped, stop processing
                        should_call_handler = false;
                        break;
                    }
                }
                Err(e) => {
                    // Forward error to receiver
                    let _ = tx.send(Err(e)).await;
                    should_call_handler = false;
                    break;
                }
            }
        }

        // Signal whether handler should be called
        let _ = handler_tx.send(should_call_handler);
    });

    // Wait for signal from streaming task
    let should_call_handler = match handler_rx.await {
        Ok(should) => should,
        Err(_) => {
            // Streaming task was dropped unexpectedly
            return Ok((StatusCode::INTERNAL_SERVER_ERROR, "Internal error").into_response());
        }
    };

    // Don't call handler if limit was exceeded
    if !should_call_handler {
        return Ok((StatusCode::PAYLOAD_TOO_LARGE, "Payload too large").into_response());
    }

    // Create a new body from the receiver stream
    let limited_body = Body::from_stream(ReceiverStream::new(rx));
    let req = Request::from_parts(parts, limited_body);

    // Call the next middleware/handler
    let response = next.run(req).await;

    // Double-check limit flag after handler completes
    if limit_exceeded.load(std::sync::atomic::Ordering::SeqCst) {
        return Ok((StatusCode::PAYLOAD_TOO_LARGE, "Payload too large").into_response());
    }

    Ok(response)
}