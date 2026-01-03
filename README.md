# 🚦 Axum Size Limit Layer

A flexible, content-type–aware **request body size limiter** for **Axum / Tower**.

Supports:
- ✅ Human-friendly sizes (`10MB`, `1.5GiB`, `100Mbit`)
- ✅ MIME type–specific limits
- ✅ Wildcards (`image/*`, `application/*`)
- ✅ Tower `Layer` + `Service`
- ✅ Clean modular architecture
- ✅ Library + example binary

---

## ✨ Features

- 📦 Per-`Content-Type` size limits
- 🧠 Sensible defaults out of the box
- 🛠 Builder-style configuration
- 🧵 Thread-safe (`Arc` based)
- 🧪 Fully testable (pure Rust logic separated)
- ⚙️ Works with Axum 0.8 + Tower 0.5

---

Usage examples:

Global limit:

```rust
use axum::{Router, routing::post};
use axum_size_limit::{SizeLimitLayer, SizeLimitConfig};

let config = SizeLimitConfig::default()
    .with_default_limit("10MB")
    .with_specific_limit("application/json", "2MB")
    .with_wildcard_limit("image/*", "20MB");

let app = Router::new()
    .route("/upload", post(upload_handler))
    .layer(SizeLimitLayer::new(config));

```
Per route limit:
```rust
use axum::{Router, routing::post};
use axum_size_limit::{SizeLimitLayer, SizeLimitConfig};

let upload_config = SizeLimitConfig::default()
    .with_default_limit("50MB")
    .with_wildcard_limit("image/*", "20MB")
    .with_specific_limit("multipart/form-data", "50MB");

let json_config = SizeLimitConfig::default()
    .with_default_limit("2MB")
    .with_specific_limit("application/json", "2MB");

let raw_config = SizeLimitConfig::default()
    .with_default_limit("100MB")
    .with_specific_limit("application/octet-stream", "100MB");

let app = Router::new()
    // Image / multipart uploads
    .route(
        "/upload",
        post(upload_handler)
            .layer(SizeLimitLayer::new(upload_config)),
    )

    // JSON-only API endpoint
    .route(
        "/api",
        post(api_handler)
            .layer(SizeLimitLayer::new(json_config)),
    )

    // Raw binary uploads
    .route(
        "/raw",
        post(raw_upload_handler)
            .layer(SizeLimitLayer::new(raw_config)),
    );

```

```rust
Router::new()
    .nest(
        "/admin",
        admin_router.layer(SizeLimitLayer::new(admin_config)),
    );
```

Run tests:
`cargo test -- --nocapture`