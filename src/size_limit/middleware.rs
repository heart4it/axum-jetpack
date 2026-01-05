// middleware.rs
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

#[derive(Clone, Debug)]
pub struct BufferStrategy {
    /// Content types that should be buffered (e.g., ["application/json", "application/*"])
    pub buffered_types: Vec<String>,
    /// Content types that should be streamed (e.g., ["video/*", "image/*"])
    pub streamed_types: Vec<String>,
    /// Default behavior for types not in either list
    pub default_is_buffered: bool,
}

impl BufferStrategy {
    pub fn new() -> Self {
        Self {
            buffered_types: Vec::new(),
            streamed_types: Vec::new(),
            default_is_buffered: false,
        }
    }

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
            default_is_buffered: false,
        }
    }

    pub fn all_buffered() -> Self {
        Self {
            buffered_types: Vec::new(),
            streamed_types: Vec::new(),
            default_is_buffered: true,
        }
    }

    pub fn all_streamed() -> Self {
        Self {
            buffered_types: Vec::new(),
            streamed_types: Vec::new(),
            default_is_buffered: false,
        }
    }

    pub fn with_buffered_types(mut self, types: &[&str]) -> Self {
        self.buffered_types
            .extend(types.iter().map(|s| s.to_string()));
        self
    }

    pub fn with_streamed_types(mut self, types: &[&str]) -> Self {
        self.streamed_types
            .extend(types.iter().map(|s| s.to_string()));
        self
    }

    pub fn with_default_buffered(mut self, is_buffered: bool) -> Self {
        self.default_is_buffered = is_buffered;
        self
    }

    pub fn clear_buffered_types(&mut self) {
        self.buffered_types.clear();
    }

    pub fn clear_streamed_types(&mut self) {
        self.streamed_types.clear();
    }

    pub fn clear_all_types(&mut self) {
        self.buffered_types.clear();
        self.streamed_types.clear();
    }

    pub fn should_buffer(&self, content_type: &str) -> bool {
        let ct_lower = content_type.to_lowercase();
        let ct_trimmed = ct_lower.split(';').next().unwrap_or(&ct_lower).trim();

        // Check exact matches first
        if self.buffered_types.iter().any(|t| t == ct_trimmed) {
            return true;
        }
        if self.streamed_types.iter().any(|t| t == ct_trimmed) {
            return false;
        }

        // Check wildcard matches
        if let Some(slash_pos) = ct_trimmed.find('/') {
            let wildcard = format!("{}/*", &ct_trimmed[..slash_pos]);

            if self.buffered_types.iter().any(|t| t == &wildcard) {
                return true;
            }
            if self.streamed_types.iter().any(|t| t == &wildcard) {
                return false;
            }
        }

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

        self.default_is_buffered
    }
}

impl Default for BufferStrategy {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[derive(Clone)]
pub struct SizeLimitMiddlewareConfig {
    /// Size limits per content type
    pub size_limits: SizeLimitConfig,
    /// Buffering vs streaming strategy
    pub buffer_strategy: BufferStrategy,
}

impl SizeLimitMiddlewareConfig {
    pub fn new(size_limits: SizeLimitConfig) -> Self {
        Self {
            size_limits,
            buffer_strategy: BufferStrategy::new(),
        }
    }

    pub fn with_default_buffer_strategy(size_limits: SizeLimitConfig) -> Self {
        Self {
            size_limits,
            buffer_strategy: BufferStrategy::with_defaults(),
        }
    }

    pub fn with_buffer_strategy(mut self, strategy: BufferStrategy) -> Self {
        self.buffer_strategy = strategy;
        self
    }

    pub fn with_buffered_types(mut self, types: &[&str]) -> Self {
        self.buffer_strategy = self.buffer_strategy.with_buffered_types(types);
        self
    }

    pub fn with_streamed_types(mut self, types: &[&str]) -> Self {
        self.buffer_strategy = self.buffer_strategy.with_streamed_types(types);
        self
    }

    pub fn with_default_buffered(mut self, is_buffered: bool) -> Self {
        self.buffer_strategy = self.buffer_strategy.with_default_buffered(is_buffered);
        self
    }
}

impl Default for SizeLimitMiddlewareConfig {
    fn default() -> Self {
        Self {
            size_limits: SizeLimitConfig::default(),
            buffer_strategy: BufferStrategy::with_defaults(),
        }
    }
}

pub fn with_size_limit(router: Router, config: SizeLimitMiddlewareConfig) -> Router {
    let config = Arc::new(config);

    router.layer(middleware::from_fn_with_state(
        config,
        |State(config): State<Arc<SizeLimitMiddlewareConfig>>, req: Request<Body>, next: Next| async move {
            let content_type = req.headers()
                .get(axum::http::header::CONTENT_TYPE)
                .and_then(|h| h.to_str().ok())
                .unwrap_or("application/octet-stream");

            let limit = config.size_limits.get_limit_for_content_type(content_type);


            // 1. Quick Content-Length check (if available)
            if let Some(content_length) = req.headers().get(axum::http::header::CONTENT_LENGTH) {
                if let Ok(length_str) = content_length.to_str() {
                    if let Ok(content_length_value) = length_str.parse::<usize>() {
                        if content_length_value > limit {
                            return Ok((StatusCode::PAYLOAD_TOO_LARGE, "Payload too large").into_response());
                        }
                    }
                }
            }

            if config.buffer_strategy.should_buffer(content_type) {
                buffer_with_limit(req, next, limit).await
            } else {
                stream_with_limit(req, next, limit).await
            }
        }
    ))
}

pub fn with_size_limit_simple(router: Router, size_limits: SizeLimitConfig) -> Router {
    let config = SizeLimitMiddlewareConfig::new(size_limits);
    with_size_limit(router, config)
}
async fn buffer_with_limit(
    mut req: Request<Body>,
    next: Next,
    max_size: usize,
) -> Result<Response, StatusCode> {
    use axum::response::IntoResponse;

    let body = std::mem::take(req.body_mut());

    match to_bytes(body, max_size).await {
        Ok(bytes) => {
            // Check actual size (to_bytes might read exactly max_size)
            if bytes.len() > max_size {
                return Ok((StatusCode::PAYLOAD_TOO_LARGE, "Payload too large").into_response());
            }

            *req.body_mut() = Body::from(bytes);
            Ok(next.run(req).await)
        }
        Err(_) => {
            // Body exceeded limit or other error
            Ok((StatusCode::PAYLOAD_TOO_LARGE, "Payload too large").into_response())
        }
    }
}

async fn stream_with_limit(
    req: Request<Body>,
    next: Next,
    max_size: usize,
) -> Result<Response, StatusCode> {
    use axum::response::IntoResponse;

    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Bytes, axum::Error>>(32);
    let (parts, body) = req.into_parts();

    let limit_exceeded = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let limit_exceeded_clone = limit_exceeded.clone();

    let (handler_tx, handler_rx) = tokio::sync::oneshot::channel::<bool>();

    tokio::spawn(async move {
        let mut stream = body.into_data_stream();
        let mut total_size = 0usize;
        let mut should_call_handler = true;

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    total_size += chunk.len();

                    if total_size > max_size {
                        limit_exceeded_clone.store(true, std::sync::atomic::Ordering::SeqCst);
                        should_call_handler = false;
                        break;
                    }

                    if tx.send(Ok(chunk)).await.is_err() {
                        should_call_handler = false;
                        break;
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(e)).await;
                    should_call_handler = false;
                    break;
                }
            }
        }

        let _ = handler_tx.send(should_call_handler);
    });

    let should_call_handler = match handler_rx.await {
        Ok(should) => should,
        Err(_) => {
            // Return full response with body
            return Ok((StatusCode::INTERNAL_SERVER_ERROR, "Internal error").into_response());
        }
    };

    if !should_call_handler {
        return Ok((StatusCode::PAYLOAD_TOO_LARGE, "Payload too large").into_response());
    }

    let limited_body = Body::from_stream(ReceiverStream::new(rx));
    let req = Request::from_parts(parts, limited_body);

    let response = next.run(req).await;

    if limit_exceeded.load(std::sync::atomic::Ordering::SeqCst) {
        return Ok((StatusCode::PAYLOAD_TOO_LARGE, "Payload too large").into_response());
    }

    Ok(response)
}
