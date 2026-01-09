# Axum Jetpack

This project should be considered experimental for now.
It contains a collection of tools to improve the axum framework and security aspects.

Currently it contains the following features:
* Size limit middleware: A configurable middleware for Axum framework that enforces request size limits with intelligent buffering and streaming strategies.
  ## Features 
  * **Content-Type Based Limits** - Set different limits for different content types
  * **Wildcard Support** - Use patterns like `image/*` or `video/*`
  * **Buffering Strategy** - Intelligent decision to buffer or stream
  * **Human-Readable Sizes** - Use strings like "10MB" or "100KB"
  * **Early soft rejection** - First weak rejection based on Content-Length header
  * **Early hard rejection** - Counts bytes and disallows request if limit is exceeded.
  * **Streaming Support** - Handle large files without buffering
  * **Customizable Defaults** - Configure default behavior
  * **Multipart Support** - Handle file upload limits
  * **Production Ready** - Proper error handling and responses

  ## Important notes:
  * This middleware is only effective when also other axum limits set correctly.
  Like: Timeouts, Max concurrent connections, CORS etc.
  * Notice that also default axum limits are enforced which can conflict with this middleware.
  Make sure to set them correctly.

## Installation

```toml
[dependencies]
axum-jetpack = "0.8.1" # currently supported axum version
```

## Usage example

Open [src/example/example.rs](src/example/example.rs)

## Run tests:
`cargo test -- --nocapture`