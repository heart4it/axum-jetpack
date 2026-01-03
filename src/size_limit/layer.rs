use axum::{body::Body, extract::Request, response::Response, BoxError};
use crate::size_limit::{SizeLimitConfig};
use crate::size_limit::service::SizeLimitService;

#[derive(Clone)]
pub struct SizeLimitLayer {
    config: SizeLimitConfig,
}

impl SizeLimitLayer {
    pub fn new(config: SizeLimitConfig) -> Self {
        Self { config }
    }
}

impl<S> tower::Layer<S> for SizeLimitLayer
where
    S: tower::Service<Request<Body>, Response = Response> + Clone + Send + Sync + 'static,
    S::Error: Into<BoxError>,
    S::Future: Send + 'static,
{
    type Service = SizeLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        SizeLimitService::new(inner, self.config.clone())
    }
}
