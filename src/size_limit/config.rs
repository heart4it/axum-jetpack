use std::collections::HashMap;
use super::size::{parse_human_size, SizeLimit};
use super::error::ErrorFormat;
use std::sync::Arc;
use crate::size_limit::SizeLimitError;

/// Size limit configuration with wildcard and specific MIME type support
#[derive(Clone)]
pub struct SizeLimitConfig {
    /// Default limit for any content type not explicitly configured
    pub default_limit: usize,
    /// Specific limits for MIME types (exact matches)
    pub specific_limits: HashMap<String, usize>,
    /// Wildcard limits (e.g., "image/*", "application/*")
    pub wildcard_limits: HashMap<String, usize>,
    /// Error response format
    pub error_format: Arc<ErrorFormat>,
}

impl Default for SizeLimitConfig {
    fn default() -> Self {
        let mut specific_limits = HashMap::new();
        let mut wildcard_limits = HashMap::new();

        // Set some reasonable defaults using human-friendly sizes
        specific_limits.insert("multipart/form-data".to_string(), parse_human_size("50MB").unwrap());
        specific_limits.insert("application/octet-stream".to_string(), parse_human_size("100MB").unwrap());
        specific_limits.insert("application/json".to_string(), parse_human_size("10MB").unwrap());
        specific_limits.insert("application/xml".to_string(), parse_human_size("10MB").unwrap());
        specific_limits.insert("text/plain".to_string(), parse_human_size("10MB").unwrap());

        // Wildcard defaults
        wildcard_limits.insert("image/*".to_string(), parse_human_size("20MB").unwrap());
        wildcard_limits.insert("video/*".to_string(), parse_human_size("100MB").unwrap());
        wildcard_limits.insert("audio/*".to_string(), parse_human_size("50MB").unwrap());
        wildcard_limits.insert("application/*".to_string(), parse_human_size("10MB").unwrap());

        Self {
            default_limit: parse_human_size("10MB").unwrap(), // 10 MB default
            specific_limits,
            wildcard_limits,
            error_format: Arc::new(ErrorFormat::default()),
        }
    }
}

impl SizeLimitConfig {
    /// Get the size limit for a specific content type
    pub fn get_limit_for_content_type(&self, content_type: &str) -> usize {
        let ct_lower = content_type.to_lowercase();
        let ct_trimmed = ct_lower.split(';').next().unwrap_or(&ct_lower).trim();

        // 1. Check for exact match
        if let Some(limit) = self.specific_limits.get(ct_trimmed) {
            return *limit;
        }

        // 2. Check for wildcard match
        if let Some(slash_pos) = ct_trimmed.find('/') {
            let wildcard = format!("{}/*", &ct_trimmed[..slash_pos]);
            if let Some(limit) = self.wildcard_limits.get(&wildcard) {
                return *limit;
            }
        }

        self.default_limit
    }

    /// Builder-style method to set default limit
    pub fn with_default_limit(mut self, limit: impl Into<SizeLimit>) -> Self {
        self.default_limit = limit.into().0;
        self
    }

    /// Builder-style method to add a specific limit
    pub fn with_specific_limit(mut self, mime_type: &str, limit: impl Into<SizeLimit>) -> Self {
        self.specific_limits.insert(mime_type.to_lowercase(), limit.into().0);
        self
    }

    /// Builder-style method to add a wildcard limit
    pub fn with_wildcard_limit(mut self, wildcard: &str, limit: impl Into<SizeLimit>) -> Self {
        self.wildcard_limits.insert(wildcard.to_lowercase(), limit.into().0);
        self
    }

    /// Create from a simple configuration string
    pub fn from_toml(config_str: &str) -> Result<Self, String> {
        // Simple TOML-like parsing for demonstration
        let mut config = SizeLimitConfig {
            default_limit: parse_human_size("10MB").unwrap(),
            specific_limits: HashMap::new(),
            wildcard_limits: HashMap::new(),
            error_format: Arc::new(ErrorFormat::default()),
        };

        for line in config_str.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"').trim_matches('\'');

                match key {
                    "default" => config.default_limit = parse_human_size(value)?,
                    "multipart" => { config.specific_limits.insert("multipart/form-data".to_string(), parse_human_size(value)?); }
                    "json" => { config.specific_limits.insert("application/json".to_string(), parse_human_size(value)?); }
                    "xml" => { config.specific_limits.insert("application/xml".to_string(), parse_human_size(value)?); }
                    "text" => { config.specific_limits.insert("text/plain".to_string(), parse_human_size(value)?); }
                    "image" => { config.wildcard_limits.insert("image/*".to_string(), parse_human_size(value)?); }
                    "video" => { config.wildcard_limits.insert("video/*".to_string(), parse_human_size(value)?); }
                    "audio" => { config.wildcard_limits.insert("audio/*".to_string(), parse_human_size(value)?); }
                    "application" => { config.wildcard_limits.insert("application/*".to_string(), parse_human_size(value)?); }
                    _ => {
                        // Try to parse as MIME type
                        if key.contains('/') {
                            config.specific_limits.insert(key.to_string(), parse_human_size(value)?);
                            ()
                        } else {
                            return Err(format!("Unknown config key: {}", key));
                        }
                    }
                };
            }
        }

        Ok(config)
    }

    /// Builder-style method to set custom error format
    pub fn with_error_format(mut self, error_format: ErrorFormat) -> Self {
        self.error_format = Arc::new(error_format);
        self
    }

    /// Shortcut for JSON:API error format
    pub fn with_json_api_errors(self) -> Self {
        self.with_error_format(ErrorFormat::JsonApi)
    }

    /// Shortcut for plain text error format
    pub fn with_plain_text_errors(self) -> Self {
        self.with_error_format(ErrorFormat::PlainText)
    }

    /// Shortcut for custom error handler
    pub fn with_custom_error_handler<F>(self, handler: F) -> Self
    where
        F: Fn(SizeLimitError) -> axum::response::Response + Send + Sync + 'static,
    {
        self.with_error_format(ErrorFormat::Custom(Box::new(handler)))
    }
}