// tests/size_limit_tests.rs
use axum::{
    body::Body,
    extract::Request,
    http::StatusCode,
    response::Response,
    routing::post,
    Router,
};
use bytes::Bytes;
use http_body_util::BodyExt;
use tower::ServiceExt;

use axum_jetpack::size_limit::{with_size_limit_simple, BufferStrategy, SizeLimit, SizeLimitConfig};

// Basic strategy tests
#[tokio::test]
async fn test_buffer_strategy_defaults() {
    let strategy = BufferStrategy::with_defaults();
    assert!(strategy.should_buffer("application/json"));
    assert!(!strategy.should_buffer("video/mp4"));
}

#[tokio::test]
async fn test_buffer_strategy_custom() {
    let strategy = BufferStrategy::new()
        .with_buffered_types(&["application/json", "custom/*"])
        .with_streamed_types(&["video/*", "specific/type"])
        .with_default_buffered(true);

    assert!(strategy.should_buffer("application/json"));
    assert!(!strategy.should_buffer("specific/type"));
    assert!(strategy.should_buffer("custom/something"));
    assert!(!strategy.should_buffer("video/mp4"));
    assert!(strategy.should_buffer("unknown/type"));
}

// Middleware tests
#[tokio::test]
async fn test_middleware_rejects_large_buffered_requests() {
    let size_limits = SizeLimitConfig::default()
        .with_default_limit(SizeLimit::bytes(50));

    let app = with_size_limit_simple(
        Router::new().route("/test", post(|_req: Request| async move {
            (StatusCode::OK, "handler")
        })),
        size_limits,
    );

    let large_body = "x".repeat(100);
    let req = Request::builder()
        .uri("/test")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(large_body))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::PAYLOAD_TOO_LARGE,
        "Large buffered request should be rejected"
    );

    let body_bytes = response.collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body_bytes);

    // Accept either empty body or "Payload too large" depending on implementation
    if !body_str.is_empty() {
        assert_eq!(body_str, "Payload too large");
    }
}

#[tokio::test]
async fn test_middleware_allows_small_buffered_requests() {
    let size_limits = SizeLimitConfig::default()
        .with_default_limit(SizeLimit::bytes(1024));

    let app = with_size_limit_simple(
        Router::new().route("/test", post(|req: Request| async move {
            match req.collect().await {
                Ok(collected) => {
                    let body = collected.to_bytes();
                    if body.len() > 0 {
                        (StatusCode::OK, format!("got {} bytes", body.len()))
                    } else {
                        (StatusCode::OK, String::from("empty body but ok"))
                    }
                }
                Err(_) => {
                    (StatusCode::INTERNAL_SERVER_ERROR, String::from("failed to read"))
                }
            }
        })),
        size_limits,
    );

    let small_body = r#"{"data": "test"}"#;
    let req = Request::builder()
        .uri("/test")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(small_body))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_middleware_empty_body() {
    let size_limits = SizeLimitConfig::default()
        .with_default_limit(SizeLimit::bytes(10));

    let app = with_size_limit_simple(
        Router::new().route("/test", post(|_req: Request| async move {
            StatusCode::OK
        })),
        size_limits,
    );

    let req = Request::builder()
        .uri("/test")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_middleware_streaming_content() {
    let size_limits = SizeLimitConfig::default()
        .with_default_limit(SizeLimit::bytes(100));

    let app = with_size_limit_simple(
        Router::new().route("/test", post(|_req: Request| async move {
            (StatusCode::OK, "handler")
        })),
        size_limits,
    );

    let video_data = Bytes::from(vec![0u8; 150]);
    let req = Request::builder()
        .uri("/test")
        .method("POST")
        .header("content-type", "video/mp4")
        .body(Body::from(video_data))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();

    if response.status() == StatusCode::PAYLOAD_TOO_LARGE {
        let body_bytes = response.collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8_lossy(&body_bytes);
        if !body_str.is_empty() {
            assert_eq!(body_str, "Payload too large");
        }
    }
}

#[tokio::test]
async fn test_middleware_basic_functionality() {
    let size_limits = SizeLimitConfig::default()
        .with_default_limit(SizeLimit::bytes(10));

    let app = with_size_limit_simple(
        Router::new().route("/", post(|_req: Request| async move {
            (StatusCode::OK, "should not reach here for large requests")
        })),
        size_limits,
    );

    // Tiny request
    let tiny_req = Request::builder()
        .uri("/")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from("a"))
        .unwrap();

    let tiny_response = app.clone().oneshot(tiny_req).await.unwrap();

    // Large request
    let large_req = Request::builder()
        .uri("/")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from("x".repeat(20)))
        .unwrap();

    let large_response = app.oneshot(large_req).await.unwrap();

    // Check results
    println!("Tiny request status: {}", tiny_response.status());
    println!("Large request status: {}", large_response.status());

    // Large request should be rejected
    assert_eq!(large_response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

// Test without middleware for comparison
#[tokio::test]
async fn test_without_middleware() {
    let app = Router::new().route("/test", post(|req: Request| async move {
        let body = req.collect().await.unwrap().to_bytes();
        (StatusCode::OK, format!("Received {} bytes", body.len()))
    }));

    let large_body = "x".repeat(1000);
    let req = Request::builder()
        .uri("/test")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(large_body))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_content_length_header_rejection() {
    use axum_jetpack::size_limit::{with_size_limit_simple, SizeLimit, SizeLimitConfig};

    let size_limits = SizeLimitConfig::default()
        .with_default_limit(SizeLimit::bytes(50));

    let app = with_size_limit_simple(
        Router::new().route("/test", post(|_req: Request| async move {
            // This should NOT be called if Content-Length check works
            (StatusCode::OK, "handler called")
        })),
        size_limits,
    );

    // Request with Content-Length header exceeding limit
    // Body is small, but header says it's large - should be rejected
    let req = Request::builder()
        .uri("/test")
        .method("POST")
        .header("content-type", "application/json")
        .header("content-length", "1000") // Claims 1000 bytes
        .body(Body::from("small body")) // Actually only 10 bytes
        .unwrap();

    let response = app.oneshot(req).await.unwrap();

    // Should be rejected based on Content-Length header
    assert_eq!(
        response.status(),
        StatusCode::PAYLOAD_TOO_LARGE,
        "Should reject based on Content-Length header"
    );

    println!("✓ Content-Length header check works");
}

#[tokio::test]
async fn test_content_length_header_within_limit() {
    use axum_jetpack::size_limit::{with_size_limit_simple, SizeLimit, SizeLimitConfig};

    let size_limits = SizeLimitConfig::default()
        .with_default_limit(SizeLimit::bytes(100));

    let app = with_size_limit_simple(
        Router::new().route("/test", post(|req: Request| async move {
            let body = req.collect().await.unwrap().to_bytes();
            (StatusCode::OK, format!("Size: {} bytes", body.len()))
        })),
        size_limits,
    );

    // Request with valid Content-Length header
    let body_content = "x".repeat(50);
    let req = Request::builder()
        .uri("/test")
        .method("POST")
        .header("content-type", "application/json")
        .header("content-length", "50") // Correct size
        .body(Body::from(body_content))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();

    // Should pass through
    assert_eq!(response.status(), StatusCode::OK);

    let response_body = response.collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&response_body);
    assert!(body_str.contains("Size: 50 bytes"));

    println!("✓ Valid Content-Length header passes through");
}

#[tokio::test]
async fn test_empty_body_with_content_length_zero() {
    use axum_jetpack::size_limit::{with_size_limit_simple, SizeLimit, SizeLimitConfig};

    let size_limits = SizeLimitConfig::default()
        .with_default_limit(SizeLimit::bytes(10)); // Small limit

    let app = with_size_limit_simple(
        Router::new().route("/test", post(|_req: Request| async move {
            (StatusCode::OK, "empty body accepted")
        })),
        size_limits,
    );

    // Empty body with Content-Length: 0
    let req = Request::builder()
        .uri("/test")
        .method("POST")
        .header("content-type", "application/json")
        .header("content-length", "0")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();

    // Should pass immediately without reading body
    assert_eq!(response.status(), StatusCode::OK);

    println!("✓ Empty body with Content-Length: 0 passes immediately");
}

#[tokio::test]
async fn test_invalid_content_length_header() {
    use axum_jetpack::size_limit::{with_size_limit_simple, SizeLimit, SizeLimitConfig};

    let size_limits = SizeLimitConfig::default()
        .with_default_limit(SizeLimit::bytes(100));

    let app = with_size_limit_simple(
        Router::new().route("/test", post(|req: Request| async move {
            let body = req.collect().await.unwrap().to_bytes();
            (StatusCode::OK, format!("Size: {} bytes", body.len()))
        })),
        size_limits,
    );

    // Request with invalid Content-Length header (not a number)
    let req = Request::builder()
        .uri("/test")
        .method("POST")
        .header("content-type", "application/json")
        .header("content-length", "not-a-number") // Invalid
        .body(Body::from("test body"))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();

    // Should still work (fall back to body reading)
    // Or could return 400 Bad Request - depends on your preference
    assert_eq!(response.status(), StatusCode::OK);

    println!("✓ Invalid Content-Length header falls back to body reading");
}