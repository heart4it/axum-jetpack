use std::collections::HashMap;
use axum::{
    body::Body,
    extract::Request,
    routing::post,
    Router,
};
use axum_jetpack::size_limit::{ErrorFormat, SizeLimitConfig, SizeLimitLayer};
use std::time::Duration;
use tokio::time::timeout;
use tower::ServiceExt;
#[tokio::test]
async fn test_builder_pattern() {
    println!("\n=== Testing builder pattern ===");

    let config = SizeLimitConfig {
        default_limit: 500,
        specific_limits: HashMap::new(),
        wildcard_limits: HashMap::new(),
        error_format: std::sync::Arc::new(ErrorFormat::default()),
    }
        .with_specific_limit("application/custom", 1000)
        .with_wildcard_limit("video/*", 2000);

    assert_eq!(config.get_limit_for_content_type("text/plain"), 500);
    assert_eq!(config.get_limit_for_content_type("application/custom"), 1000);
    assert_eq!(config.get_limit_for_content_type("video/mp4"), 2000);
    assert_eq!(config.get_limit_for_content_type("video/avi"), 2000);

    println!("✅ Builder pattern works correctly");

    let config2 = SizeLimitConfig {
        default_limit: 100,
        specific_limits: HashMap::new(),
        wildcard_limits: HashMap::new(),
        error_format: std::sync::Arc::new(ErrorFormat::default()),
    }
        .with_wildcard_limit("application/*", 500)
        .with_specific_limit("application/json", 1000);

    assert_eq!(config2.get_limit_for_content_type("application/xml"), 500);
    assert_eq!(config2.get_limit_for_content_type("application/json"), 1000);

    println!("✅ Specific limits override wildcards");
}

#[tokio::test]
async fn test_simple_size_check() {
    println!("\n=== Simple size check tests ===");

    let config = SizeLimitConfig {
        default_limit: 100,
        specific_limits: HashMap::new(),
        wildcard_limits: HashMap::new(),
        error_format: std::sync::Arc::new(ErrorFormat::default()),
    }
        .with_specific_limit("application/json", 150)
        .with_wildcard_limit("image/*", 200);

    let size_limit_layer = SizeLimitLayer::new(config);

    let app = Router::new()
        .route("/test", post(|body: Body| async move {
            match axum::body::to_bytes(body, 1000).await {
                Ok(bytes) => format!("Got {} bytes", bytes.len()),
                Err(e) => format!("Error: {}", e),
            }
        }))
        .layer(size_limit_layer);

    let body_123 = vec![0u8; 123];

    let test_cases = vec![
        (None, 413, "no content-type (over 100 default)"),
        (Some("text/plain"), 413, "text/plain (over 100 default)"),
        (Some("application/json"), 200, "application/json (123 < 150)"),
        (Some("image/jpeg"), 200, "image/jpeg (123 < 200)"),
        (Some("image/png"), 200, "image/png (123 < 200)"),
    ];

    for (content_type, expected_status, description) in test_cases {
        println!("\nTest 123 bytes: {}", description);

        let mut request_builder = Request::builder()
            .method("POST")
            .uri("/test");

        if let Some(ct) = content_type {
            request_builder = request_builder.header("content-type", ct);
        }

        let request = request_builder
            .body(Body::from(body_123.clone()))
            .unwrap();

        let response = timeout(Duration::from_secs(2), app.clone().oneshot(request))
            .await
            .expect("Request timed out")
            .unwrap();

        let status = response.status();
        let response_body = axum::body::to_bytes(response.into_body(), 1000).await.unwrap();
        let response_text = String::from_utf8_lossy(&response_body);

        println!("  Status: {}, Response: {}", status, response_text);

        if expected_status == 413 {
            assert_eq!(status, 413, "Should be rejected with 413 Payload Too Large");
            assert!(response_text.contains("Payload too large") || response_text.contains("Body too large"),
                    "Should contain error message. Got: {}", response_text);
            println!("  ✅ Correctly rejected (413 Payload Too Large)");
        } else {
            assert_eq!(status, 200, "Should succeed with 200");
            assert!(response_text.contains("123"),
                    "Should contain 123");
            println!("  ✅ Success");
        }
    }
}

#[tokio::test]
async fn test_wildcard_matching() {
    println!("\n=== Testing wildcard matching ===");

    let config = SizeLimitConfig {
        default_limit: 100,
        specific_limits: HashMap::new(),
        wildcard_limits: HashMap::new(),
        error_format: std::sync::Arc::new(ErrorFormat::default()),
    }
        .with_wildcard_limit("image/*", 200)
        .with_wildcard_limit("application/*", 300)
        .with_specific_limit("application/json", 150);

    let size_limit_layer = SizeLimitLayer::new(config);

    let app = Router::new()
        .route("/upload", post(|body: Body| async move {
            match axum::body::to_bytes(body, 1000).await {
                Ok(bytes) => format!("Got {} bytes", bytes.len()),
                Err(e) => format!("Error: {}", e),
            }
        }))
        .layer(size_limit_layer);

    let test_cases = vec![
        ("image/jpeg", 180, 200, true, "Image wildcard (180 < 200)"),
        ("image/jpeg", 220, 200, false, "Image wildcard (220 > 200)"),
        ("image/png", 190, 200, true, "Another image type"),
        ("application/pdf", 280, 300, true, "Application wildcard (280 < 300)"),
        ("application/pdf", 320, 300, false, "Application wildcard (320 > 300)"),
        ("application/json", 140, 150, true, "Specific JSON limit (140 < 150)"),
        ("application/json", 160, 150, false, "Specific JSON limit (160 > 150)"),
        ("text/plain", 90, 100, true, "Default limit (90 < 100)"),
        ("text/plain", 110, 100, false, "Default limit (110 > 100)"),
        ("unknown/type", 95, 100, true, "Unknown type uses default"),
        ("unknown/type", 105, 100, false, "Unknown type uses default"),
    ];

    for (content_type, size_bytes, expected_limit, should_succeed, description) in test_cases {
        println!("\nTest: {}", description);

        let body = vec![0u8; size_bytes];
        let request = Request::builder()
            .method("POST")
            .uri("/upload")
            .header("content-type", content_type)
            .body(Body::from(body))
            .unwrap();

        let response = timeout(Duration::from_secs(2), app.clone().oneshot(request))
            .await
            .expect("Request timed out")
            .unwrap();

        let status = response.status();
        let response_body = axum::body::to_bytes(response.into_body(), 1000).await.unwrap();
        let response_text = String::from_utf8_lossy(&response_body);

        println!("  Status: {}, Response: {}", status, response_text);

        if should_succeed {
            assert_eq!(status, 200, "Should succeed with 200");
            assert!(response_text.contains(&format!("{}", size_bytes)),
                    "Should contain byte count");
            println!("  ✅ Success");
        } else {
            assert_eq!(status, 413, "Should be rejected with 413 Payload Too Large");
            assert!(response_text.contains("Payload too large") || response_text.contains("Body too large"),
                    "Should contain error message. Got: {}", response_text);
            println!("  ✅ Correctly rejected (413 Payload Too Large)");
        }
    }
}

#[tokio::test]
async fn test_priority_matching() {
    println!("\n=== Testing matching priority ===");

    let config = SizeLimitConfig {
        default_limit: 100,
        specific_limits: HashMap::new(),
        wildcard_limits: HashMap::new(),
        error_format: std::sync::Arc::new(ErrorFormat::default()),
    }
        .with_wildcard_limit("application/*", 200)
        .with_specific_limit("application/json", 300)
        .with_specific_limit("application/pdf", 400);

    let size_limit_layer = SizeLimitLayer::new(config);

    let app = Router::new()
        .route("/upload", post(|body: Body| async move {
            match axum::body::to_bytes(body, 1000).await {
                Ok(bytes) => format!("Got {} bytes", bytes.len()),
                Err(e) => format!("Error: {}", e),
            }
        }))
        .layer(size_limit_layer);

    let test_cases = vec![
        ("application/json", 250, true, "Specific JSON limit (250 < 300)"),
        ("application/json", 350, false, "Specific JSON limit (350 > 300)"),
        ("application/pdf", 350, true, "Specific PDF limit (350 < 400)"),
        ("application/xml", 180, true, "Wildcard app limit (180 < 200)"),
        ("application/xml", 220, false, "Wildcard app limit (220 > 200)"),
    ];

    for (content_type, size_bytes, should_succeed, description) in test_cases {
        println!("\nTest: {}", description);

        let body = vec![0u8; size_bytes];
        let request = Request::builder()
            .method("POST")
            .uri("/upload")
            .header("content-type", content_type)
            .body(Body::from(body))
            .unwrap();

        let response = timeout(Duration::from_secs(2), app.clone().oneshot(request))
            .await
            .expect("Request timed out")
            .unwrap();

        let status = response.status();
        let response_body = axum::body::to_bytes(response.into_body(), 1000).await.unwrap();
        let response_text = String::from_utf8_lossy(&response_body);

        println!("  Status: {}, Response: {}", status, response_text);

        if should_succeed {
            assert_eq!(status, 200, "Should succeed with 200");
            assert!(response_text.contains(&format!("{}", size_bytes)),
                    "Should contain byte count");
            println!("  ✅ Success");
        } else {
            assert_eq!(status, 413, "Should be rejected with 413 Payload Too Large");
            assert!(response_text.contains("Payload too large") || response_text.contains("Body too large"),
                    "Should contain error message. Got: {}", response_text);
            println!("  ✅ Correctly rejected (413 Payload Too Large)");
        }
    }
}

#[tokio::test]
async fn test_streaming_all_content_types() {
    println!("\n=== Testing streaming for ALL content types ===");

    let config = SizeLimitConfig {
        default_limit: 100,
        specific_limits: HashMap::new(),
        wildcard_limits: HashMap::new(),
        error_format: std::sync::Arc::new(ErrorFormat::default()),
    }
        .with_specific_limit("application/json", 150)
        .with_wildcard_limit("image/*", 200);

    let size_limit_layer = SizeLimitLayer::new(config);

    let app = Router::new()
        .route("/api", post(|body: String| async move {
            format!("Parsed: {} chars", body.len())
        }))
        .layer(size_limit_layer);

    let test_cases = vec![
        ("application/json", 80, 200, "JSON under limit"),
        ("application/json", 160, 413, "JSON over limit - middleware rejects with 413"),
        ("text/plain", 90, 200, "Text under default limit"),
        ("text/plain", 110, 413, "Text over default limit - middleware rejects with 413"),
    ];

    for (content_type, size_bytes, expected_status, description) in test_cases {
        println!("\nTest with String handler: {}", description);

        let body = "x".repeat(size_bytes);
        let request = Request::builder()
            .method("POST")
            .uri("/api")
            .header("content-type", content_type)
            .body(Body::from(body))
            .unwrap();

        let response = timeout(Duration::from_secs(2), app.clone().oneshot(request))
            .await
            .expect("Request timed out")
            .unwrap();

        let status = response.status();
        let response_body = axum::body::to_bytes(response.into_body(), 1000).await.unwrap();
        let response_text = String::from_utf8_lossy(&response_body);

        println!("  Status: {}, Response: {}", status, response_text);

        assert_eq!(status.as_u16(), expected_status as u16,
                   "Expected status {}, got {}", expected_status, status);

        if expected_status == 200 {
            assert!(response_text.contains(&format!("{}", size_bytes)),
                    "Should contain size");
            println!("  ✅ Success");
        } else {
            assert_eq!(status, 413, "Should be rejected with 413 Payload Too Large");
            assert!(response_text.contains("Payload too large") || response_text.contains("Body too large") || response_text.contains("Failed to buffer"),
                    "Should contain error message. Got: {}", response_text);
            println!("  ✅ Correctly rejected (413 Payload Too Large)");
        }
    }
}