# Axum Jetpack

This project should be considered experimental for now.
It contains a collection of tools to improve the axum framework and security aspects.

Currently it contains the following features:
* Size limit middleware: A configurable middleware for Axum framework that enforces request size limits with intelligent buffering and streaming strategies.
  ## Features 
  1. **Content-Type Based Limits** - Set different limits for different content types
  2. **Wildcard Support** - Use patterns like `image/*` or `video/*`
  3. **Buffering Strategy** - Intelligent decision to buffer or stream
  4. **Human-Readable Sizes** - Use strings like "10MB" or "100KB"
  5. **Early soft rejection** - First weak rejection based on Content-Length header
  6  **Early hard rejection** - Counts bytes and disallows request if limit is exceeded.
  7. **Streaming Support** - Handle large files without buffering
  8. **Customizable Defaults** - Configure default behavior
  9. **Multipart Support** - Handle file upload limits
  10. **Production Ready** - Proper error handling and responses

  ## Important notes:
  * This middleware is only effective when also other axum limits set correctly.
  Like: Timeouts, Max concurrent connections, CORS etc.
  * Notice that also default axum limits are enforced which can conflict with this middleware.
  Make sure to set them correctly.

## Installation

```toml
[dependencies]
axum-jetpack = "0.8.0" # currently suppoerted axum version
```

## Usage example

Open [src/example/example.rs](src/example/example.rs)

## Run tests:
`cargo test -- --nocapture`