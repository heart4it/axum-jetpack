use axum::{
    body::Body,
    extract::Request,
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use axum_jetpack::size_limit::{with_size_limit, BufferStrategy, SizeLimitConfig, SizeLimitMiddlewareConfig};
use futures::StreamExt;
use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main() {
    // Create a CLEAN size limit configuration (no pre-configured JSON limit)
    let size_limits = SizeLimitConfig::default()
        .with_default_limit("1b")
        .with_specific_limit("multipart/form-data", "50MB")
        .with_wildcard_limit("image/*", "20MB")
        .with_wildcard_limit("video/*", "100MB");


    println!("SizeLimitConfig: {:#?}", size_limits);

    // OPTION 1: Start with empty buffer strategy and add types
    let buffer_strategy = BufferStrategy::new() // Empty
        .with_buffered_types(&[
            "application/json",
            "multipart/form-data",
            "text/*",
        ])
        .with_streamed_types(&[
            "video/*",
            "image/*",
            "audio/*",
            "application/octet-stream",
        ])
        .with_default_buffered(false); // Stream unknown types

    println!("SizeLimitConfig: {:#?}", buffer_strategy);
    // OPTION 2: Start with defaults and modify
    // let buffer_strategy = BufferStrategy::with_defaults()
    //     .with_default_buffered(false); // Just modify default behavior

    // OPTION 3: Buffer everything (not recommended for large files)
    // let buffer_strategy = BufferStrategy::all_buffered();

    // OPTION 4: Stream everything (not recommended for JSON/form data)
    // let buffer_strategy = BufferStrategy::all_streamed();

    // Combine into middleware config
    let middleware_config = SizeLimitMiddlewareConfig::new(size_limits)
        .with_buffer_strategy(buffer_strategy);

    // Create router and apply middleware
    let app = Router::new()
        .route("/api/json", post(handle_json))
        .route("/api/upload", post(handle_upload_body))
        .route("/api/data", post(handle_data));

    let app = with_size_limit(app, middleware_config);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("Listening on http://127.0.0.1:3000");
    axum::serve(listener, app.into_make_service()).await.unwrap();
}

#[derive(Deserialize, Serialize)]
struct JsonData {
    field: String,
    value: i32,
}

async fn handle_json(Json(data): Json<JsonData>) -> impl IntoResponse {
    (StatusCode::OK, format!("JSON: {} = {}", data.field, data.value))
}

async fn handle_upload_body(body: Body) -> impl IntoResponse {
    let mut stream = body.into_data_stream();
    let mut total = 0usize;

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(bytes) => total += bytes.len(),
            Err(_) => return (StatusCode::BAD_REQUEST, "Failed to read body").into_response(),
        }
    }

    format!("Uploaded: {} bytes", total).into_response()
}

async fn handle_data(req: Request<Body>) -> impl IntoResponse {
    let content_type = req.headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    let body = req.into_body();
    let mut stream = body.into_data_stream();
    let mut total = 0usize;

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(bytes) => total += bytes.len(),
            Err(_) => return (StatusCode::BAD_REQUEST, "Failed to read body").into_response(),
        }
    }

    (StatusCode::OK, format!("Received {} bytes with content-type: {}", total, content_type)).into_response()
}