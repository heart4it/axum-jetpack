use std::collections::HashMap;
use crate::size_limit::{parse_human_size, SizeLimit};

#[derive(Clone, Debug)]
pub struct SizeLimitConfig {
    /// Default limit for any content type not explicitly configured
    pub default_limit: usize,
    /// Specific limits for MIME types (exact matches) (e.g. application/json)
    pub specific_limits: HashMap<String, usize>,
    /// Wildcard limits (e.g., "image/*", "application/*")
    pub wildcard_limits: HashMap<String, usize>,
}

impl Default for SizeLimitConfig {
    fn default() -> Self {
        Self {
            default_limit: parse_human_size("1mb").unwrap(),
            specific_limits: HashMap::new(),
            wildcard_limits: HashMap::new(),
        }
    }
}

impl SizeLimitConfig {
    pub fn get_limit_for_content_type(&self, content_type: &str) -> usize {
        let ct_lower = content_type.to_lowercase();
        let ct_trimmed = ct_lower.split(';').next().unwrap_or(&ct_lower).trim();

        if let Some(limit) = self.specific_limits.get(ct_trimmed) {
            return *limit;
        }

        if let Some(slash_pos) = ct_trimmed.find('/') {
            let wildcard = format!("{}/*", &ct_trimmed[..slash_pos]);
            if let Some(limit) = self.wildcard_limits.get(&wildcard) {
                return *limit;
            }
        }

        self.default_limit
    }

    pub fn with_default_limit(mut self, limit: impl Into<SizeLimit>) -> Self {
        self.default_limit = limit.into().0;
        self
    }

    pub fn with_specific_limit(mut self, mime_type: &str, limit: impl Into<SizeLimit>) -> Self {
        self.specific_limits.insert(mime_type.to_lowercase(), limit.into().0);
        self
    }

    pub fn with_wildcard_limit(mut self, wildcard: &str, limit: impl Into<SizeLimit>) -> Self {
        self.wildcard_limits.insert(wildcard.to_lowercase(), limit.into().0);
        self
    }
}