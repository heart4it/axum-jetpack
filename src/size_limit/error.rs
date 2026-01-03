use axum::{
    http::StatusCode,
    response::{Response, IntoResponse},
};
use serde::Serialize;

/// Simple error type
#[derive(Debug, Serialize)]
#[derive(Clone)]
pub enum SizeLimitError {
    BodyTooLarge {
        max_size: usize,
        actual_size: usize,
    },
    Other(String),
    SizeOverflow,
    ChunkTooLarge { max_chunk_size: usize, actual_chunk_size: usize },
}

impl std::fmt::Display for SizeLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SizeLimitError::BodyTooLarge { max_size, actual_size: _ } => {
                write!(f, "Body too large: Maximum size is {} bytes", max_size)
            }
            SizeLimitError::Other(msg) => write!(f, "Error: {}", msg),
            SizeLimitError::SizeOverflow => write!(f, "Size overflow error"),
            SizeLimitError::ChunkTooLarge { max_chunk_size, actual_chunk_size: _ } => {
                write!(f, "Chunk too large: Maximum chunk size is {} bytes", max_chunk_size)
            }
        }
    }
}

impl std::error::Error for SizeLimitError {}

/// Custom error response format
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    pub status_code: u16,
}

impl IntoResponse for SizeLimitError {
    fn into_response(self) -> Response {
        let (status, message, details) = match &self {
            SizeLimitError::BodyTooLarge { max_size, actual_size } => {
                (
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "Payload too large".to_string(),
                    Some(format!("Request size: {} bytes, Maximum allowed: {} bytes", actual_size, max_size)),
                )
            }
            SizeLimitError::Other(msg) => {
                (
                    StatusCode::BAD_REQUEST,
                    "Bad request".to_string(),
                    Some(msg.clone()),
                )
            }
            SizeLimitError::SizeOverflow => {
                (
                    StatusCode::BAD_REQUEST,
                    "Size overflow".to_string(),
                    Some("Request size calculation resulted in an overflow".to_string()),
                )
            }
            SizeLimitError::ChunkTooLarge { max_chunk_size, actual_chunk_size } => {
                (
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "Chunk too large".to_string(),
                    Some(format!("Chunk size: {} bytes, Maximum allowed: {} bytes", actual_chunk_size, max_chunk_size)),
                )
            }
        };

        let error_response = ErrorResponse {
            error: status.to_string(),
            message,
            details,
            status_code: status.as_u16(),
        };

        // Return as JSON
        (status, axum::Json(error_response)).into_response()
    }
}

/// JSON API error handler (for REST APIs)
#[derive(Debug, Serialize)]
pub struct JsonApiError {
    pub errors: Vec<JsonApiErrorDetail>,
}

#[derive(Debug, Serialize)]
pub struct JsonApiErrorDetail {
    pub status: String,
    pub title: String,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

/// Implement IntoResponse for different error formats
pub enum ErrorFormat {
    /// Simple JSON error format (default)
    SimpleJson,
    /// JSON:API error format
    JsonApi,
    /// Plain text error format
    PlainText,
    /// Custom error handler function
    Custom(Box<dyn Fn(SizeLimitError) -> Response + Send + Sync>),
}

impl Default for ErrorFormat {
    fn default() -> Self {
        ErrorFormat::SimpleJson
    }
}

impl ErrorFormat {
    pub fn handle_error(&self, error: SizeLimitError) -> Response {
        match self {
            ErrorFormat::SimpleJson => error.into_response(), // Uses default IntoResponse

            ErrorFormat::JsonApi => {
                let (status, title, detail, meta) = match error {
                    SizeLimitError::BodyTooLarge { max_size, actual_size } => {
                        let meta = serde_json::json!({
                            "max_size": max_size,
                            "actual_size": actual_size,
                        });
                        (
                            StatusCode::PAYLOAD_TOO_LARGE,
                            "Payload Too Large",
                            format!("Request body exceeds the maximum allowed size of {} bytes", max_size),
                            Some(meta),
                        )
                    }
                    SizeLimitError::Other(msg) => {
                        (
                            StatusCode::BAD_REQUEST,
                            "Bad Request",
                            msg,
                            None,
                        )
                    }
                    SizeLimitError::SizeOverflow => {
                        (
                            StatusCode::BAD_REQUEST,
                            "Size Overflow",
                            "Request size calculation resulted in an overflow".to_string(),
                            None,
                        )
                    }
                    SizeLimitError::ChunkTooLarge { max_chunk_size, actual_chunk_size } => {
                        let meta = serde_json::json!({
                            "max_chunk_size": max_chunk_size,
                            "actual_chunk_size": actual_chunk_size,
                        });
                        (
                            StatusCode::PAYLOAD_TOO_LARGE,
                            "Chunk Too Large",
                            format!("Request chunk exceeds the maximum allowed size of {} bytes", max_chunk_size),
                            Some(meta),
                        )
                    }
                };

                let error_detail = JsonApiErrorDetail {
                    status: status.as_u16().to_string(),
                    title: title.to_string(),
                    detail,
                    meta,
                };

                let json_api_error = JsonApiError {
                    errors: vec![error_detail],
                };

                (status, axum::Json(json_api_error)).into_response()
            }

            ErrorFormat::PlainText => {
                let (status, body) = match error {
                    SizeLimitError::BodyTooLarge { max_size, actual_size } => {
                        let body = format!(
                            "413 Payload Too Large\n\nRequest size: {} bytes\nMaximum allowed: {} bytes",
                            actual_size, max_size
                        );
                        (StatusCode::PAYLOAD_TOO_LARGE, body)
                    }
                    SizeLimitError::Other(msg) => {
                        (StatusCode::BAD_REQUEST, format!("400 Bad Request\n\n{}", msg))
                    }
                    SizeLimitError::SizeOverflow => {
                        (StatusCode::BAD_REQUEST, "400 Bad Request\n\nRequest size calculation resulted in an overflow".to_string())
                    }
                    SizeLimitError::ChunkTooLarge { max_chunk_size, actual_chunk_size } => {
                        let body = format!(
                            "413 Payload Too Large\n\nChunk size: {} bytes\nMaximum allowed: {} bytes",
                            actual_chunk_size, max_chunk_size
                        );
                        (StatusCode::PAYLOAD_TOO_LARGE, body)
                    }
                };

                (status, body).into_response()
            }

            ErrorFormat::Custom(handler) => handler(error),
        }
    }
}