use axum::{
    routing::{get, post}, Json,
    Router,
};
use axum_jetpack::size_limit::{ErrorFormat, SizeLimitConfig, SizeLimitError};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use axum::response::{Html, Response};
use axum_jetpack::size_limit::layer::SizeLimitLayer;

#[derive(Debug, Serialize, Deserialize)]
struct UploadResponse {
    message: String,
    bytes_received: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct UserData {
    username: String,
    email: String,
    bio: String,
}

#[tokio::main]
async fn main() {
    println!("=== Size Limit Middleware with Custom Error Handling ===");

    // Example 1: Default JSON error format
    let config1 = SizeLimitConfig::default()
        .with_default_limit("1MB")
        .with_specific_limit("application/json", "2MB")
        .with_wildcard_limit("image/*", "5MB");

    // Example 2: JSON:API error format
    let config2 = SizeLimitConfig::default()
        .with_default_limit("1MB")
        .with_json_api_errors();

    // Example 3: Plain text errors
    let config3 = SizeLimitConfig::default()
        .with_default_limit("1MB")
        .with_plain_text_errors();

    // Example 4: Custom HTML error handler
    let config4 = SizeLimitConfig::default()
        .with_default_limit("1MB")
        .with_custom_error_handler(|error: SizeLimitError| {
            let html = match error {
                SizeLimitError::BodyTooLarge { max_size, actual_size } => {
                    let max_mb = max_size as f64 / 1_000_000.0;
                    let actual_mb = actual_size as f64 / 1_000_000.0;

                    format!(
                        r#"<!DOCTYPE html>
                        <html>
                        <head>
                            <title>File Too Large</title>
                            <style>
                                body {{ font-family: Arial, sans-serif; margin: 40px; line-height: 1.6; }}
                                .container {{ max-width: 600px; margin: 0 auto; }}
                                .error {{ background: #fff5f5; border-left: 4px solid #e53e3e; padding: 20px; }}
                                h1 {{ color: #c53030; margin-top: 0; }}
                                .stats {{ background: #edf2f7; padding: 15px; border-radius: 4px; margin: 20px 0; }}
                                .back-btn {{ display: inline-block; background: #4299e1; color: white; padding: 10px 20px; text-decoration: none; border-radius: 4px; }}
                            </style>
                        </head>
                        <body>
                            <div class="container">
                                <div class="error">
                                    <h1>📁 File Too Large</h1>
                                    <p>The file you're trying to upload exceeds the maximum allowed size.</p>

                                    <div class="stats">
                                        <p><strong>Maximum size:</strong> {:.2} MB</p>
                                        <p><strong>Your file size:</strong> {:.2} MB</p>
                                        <p><strong>Exceeds by:</strong> {:.2} MB</p>
                                    </div>

                                    <p>Please try again with a smaller file.</p>
                                    <a href="/" class="back-btn">← Go Back</a>
                                </div>
                            </div>
                        </body>
                        </html>"#,
                        max_mb, actual_mb, actual_mb - max_mb
                    )
                }
                SizeLimitError::Other(msg) => {
                    format!(
                        r#"<!DOCTYPE html>
                        <html>
                        <head><title>Upload Error</title></head>
                        <body>
                            <h1>❌ Upload Error</h1>
                            <p>{}</p>
                            <a href="/">Go back</a>
                        </body>
                        </html>"#,
                        msg
                    )
                }
                _ => {
                    format!(
                        r#"<!DOCTYPE html>
                        <html>
                        <head><title>Request Error</title></head>
                        <body>
                            <h1>❌ Request Error</h1>
                            <p>There was a problem with your request.</p>
                            <a href="/">Go back</a>
                        </body>
                        </html>"#
                    )
                }
            };

            Response::builder()
                .status(413)
                .header("content-type", "text/html; charset=utf-8")
                .body(axum::body::Body::from(html))
                .unwrap()
        });

    // Example 5: Custom error with logging
    let config5 = SizeLimitConfig::default()
        .with_default_limit("1MB")
        .with_custom_error_handler(|error| {
            // Log to stderr
            eprintln!("[SIZE_LIMIT] {}", error);

            // Then use JSON:API format
            ErrorFormat::JsonApi.handle_error(error)
        });

    // Create different routers for demonstration
    let api_router = Router::new()
        .route("/api/register", post(register_user))
        .route("/api/upload", post(upload_handler))
        .layer(SizeLimitLayer::new(config1));

    let jsonapi_router = Router::new()
        .route("/jsonapi/upload", post(upload_handler))
        .layer(SizeLimitLayer::new(config2));

    let web_router = Router::new()
        .route("/web/upload", post(upload_handler))
        .layer(SizeLimitLayer::new(config4));

    let app = Router::new()
        .merge(api_router)
        .merge(jsonapi_router)
        .merge(web_router)
        .route("/", get(index_page))
        .route("/health", get(|| async { "OK" }));

    // Run the server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("\n🚀 Server listening on http://{}", addr);
    println!("\n📤 Test endpoints:");
    println!("  POST /api/register    - JSON API (default errors)");
    println!("  POST /api/upload      - JSON API (default errors)");
    println!("  POST /jsonapi/upload  - JSON:API format errors");
    println!("  POST /web/upload      - HTML error pages");
    println!("  GET  /                - Index page");
    println!("  GET  /health          - Health check");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn index_page() -> Html<&'static str> {
    Html(r#"<!DOCTYPE html>
    <html>
    <head>
        <title>Size Limit Demo</title>
        <style>
            body { font-family: Arial, sans-serif; margin: 40px; }
            .container { max-width: 800px; margin: 0 auto; }
            .endpoint { background: #f7fafc; padding: 20px; margin: 20px 0; border-radius: 8px; }
            code { background: #edf2f7; padding: 2px 6px; border-radius: 4px; }
            h2 { color: #2d3748; }
        </style>
    </head>
    <body>
        <div class="container">
            <h1>📏 Size Limit Middleware Demo</h1>
            <p>Test different error formats for size limit validation:</p>

            <div class="endpoint">
                <h2>1. Default JSON Errors</h2>
                <p><code>POST /api/upload</code></p>
                <p>Try: <code>curl -X POST http://localhost:3000/api/upload -H "Content-Type: text/plain" -d "$(printf 'x%.0s' {1..2000000})"</code></p>
            </div>

            <div class="endpoint">
                <h2>2. JSON:API Format</h2>
                <p><code>POST /jsonapi/upload</code></p>
                <p>Try same command with <code>/jsonapi/upload</code></p>
            </div>

            <div class="endpoint">
                <h2>3. HTML Error Pages</h2>
                <p><code>POST /web/upload</code></p>
                <p>Open browser DevTools to see HTML response</p>
            </div>
        </div>
    </body>
    </html>"#)
}

async fn register_user(Json(user): Json<UserData>) -> Json<UploadResponse> {
    Json(UploadResponse {
        message: format!("User {} registered", user.username),
        bytes_received: serde_json::to_string(&user).unwrap().len(),
    })
}

async fn upload_handler(body: String) -> Json<UploadResponse> {
    Json(UploadResponse {
        message: "Upload successful".to_string(),
        bytes_received: body.len(),
    })
}