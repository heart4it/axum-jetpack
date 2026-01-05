use std::collections::HashMap;
use crate::size_limit::{parse_human_size, SizeLimit};

/// Configuration for size limits based on content type.
///
/// This struct allows setting different size limits for different types of content,
/// with support for exact MIME type matches and wildcard patterns.
///
/// # Examples
/// ```
/// use axum_jetpack::size_limit::SizeLimitConfig;
///
/// // Create a configuration with custom limits
/// let config = SizeLimitConfig::default()
///     .with_default_limit("10mb")  // Default limit for all types
///     .with_specific_limit("application/json", "100kb")  // JSON APIs limited to 100KB
///     .with_wildcard_limit("image/*", "5mb");  // All images limited to 5MB
/// ```
#[derive(Clone, Debug)]
pub struct SizeLimitConfig {
    /// Default limit for any content type not explicitly configured.
    ///
    /// This limit applies when:
    /// 1. No exact MIME type match is found in `specific_limits`
    /// 2. No wildcard match is found in `wildcard_limits`
    ///
    /// Default value: 1 megabyte (1MB) = 1,000,000 bytes
    pub default_limit: usize,

    /// Specific limits for exact MIME type matches.
    ///
    /// These limits apply to content types that exactly match the key.
    /// Examples:
    /// - `"application/json"` → matches only `application/json`
    /// - `"text/plain"` → matches only `text/plain`
    ///
    /// The map keys should be lowercase MIME types without parameters.
    /// For example: `"application/json"` not `"application/json; charset=utf-8"`
    pub specific_limits: HashMap<String, usize>,

    /// Wildcard limits for MIME type patterns.
    ///
    /// These limits apply to content types that match wildcard patterns.
    /// Examples:
    /// - `"image/*"` → matches all image types (`image/jpeg`, `image/png`, etc.)
    /// - `"application/*"` → matches all application types
    /// - `"text/*"` → matches all text types
    ///
    /// Wildcards must follow the format `"type/*"` (asterisk after slash).
    /// The map keys should be lowercase.
    pub wildcard_limits: HashMap<String, usize>,
}

impl Default for SizeLimitConfig {
    /// Creates a default `SizeLimitConfig` with sensible defaults.
    ///
    /// Default configuration:
    /// - `default_limit`: 1 megabyte (1,000,000 bytes)
    /// - `specific_limits`: Empty (no specific limits)
    /// - `wildcard_limits`: Empty (no wildcard limits)
    ///
    /// # Examples
    /// ```
    /// use axum_jetpack::size_limit::SizeLimitConfig;
    ///
    /// let config = SizeLimitConfig::default();
    /// // 1MB = 1,000,000 bytes (decimal)
    /// assert_eq!(config.default_limit, 1_000_000);
    /// ```
    fn default() -> Self {
        Self {
            default_limit: parse_human_size("1mb").unwrap_or(1_000_000),
            specific_limits: HashMap::new(),
            wildcard_limits: HashMap::new(),
        }
    }
}

impl SizeLimitConfig {
    /// Determines the appropriate size limit for a given content type.
    ///
    /// The lookup follows this priority order:
    /// 1. **Exact match**: Check if the content type exists in `specific_limits`
    /// 2. **Wildcard match**: Check if a wildcard pattern matches in `wildcard_limits`
    /// 3. **Default**: Return `default_limit`
    ///
    /// # Arguments
    /// * `content_type` - The Content-Type header value (e.g., "application/json; charset=utf-8")
    ///
    /// # Returns
    /// The size limit in bytes for the given content type.
    ///
    /// # Examples
    /// ```
    /// use axum_jetpack::size_limit::SizeLimitConfig;
    ///
    /// let config = SizeLimitConfig::default()
    ///     .with_default_limit("2mb")
    ///     .with_specific_limit("application/json", "100kb")
    ///     .with_wildcard_limit("image/*", "5mb");
    ///
    /// // Exact match - 100KB = 100,000 bytes
    /// assert_eq!(config.get_limit_for_content_type("application/json"), 100_000);
    ///
    /// // Wildcard match - 5MB = 5,000,000 bytes
    /// assert_eq!(config.get_limit_for_content_type("image/jpeg"), 5_000_000);
    ///
    /// // Default (no match) - 2MB = 2,000,000 bytes
    /// assert_eq!(config.get_limit_for_content_type("video/mp4"), 2_000_000);
    ///
    /// // Handles content type with parameters
    /// assert_eq!(config.get_limit_for_content_type("application/json; charset=utf-8"), 100_000);
    /// ```
    pub fn get_limit_for_content_type(&self, content_type: &str) -> usize {
        // Normalize the content type: convert to lowercase and strip parameters
        let ct_lower = content_type.to_lowercase();
        let ct_trimmed = ct_lower.split(';').next().unwrap_or(&ct_lower).trim();

        // 1. Check for exact match in specific limits
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

        // 3. Fall back to default limit
        self.default_limit
    }

    /// Builder method to set the default size limit.
    ///
    /// The default limit applies to any content type that doesn't have
    /// an exact or wildcard match in the configuration.
    ///
    /// # Arguments
    /// * `limit` - The size limit, which can be:
    ///   - A string with human-readable size (e.g., "1mb", "100kb", "2.5gb")
    ///   - A `SizeLimit` struct
    ///   - A raw byte count (`usize`)
    ///
    /// # Returns
    /// `Self` for method chaining.
    ///
    /// # Examples
    /// ```
    /// use axum_jetpack::size_limit::SizeLimitConfig;
    ///
    /// let config = SizeLimitConfig::default()
    ///     .with_default_limit("10mb"); // 10 megabyte default limit = 10,000,000 bytes
    ///
    /// // Can also use raw bytes
    /// let config = SizeLimitConfig::default()
    ///     .with_default_limit(5 * 1024 * 1024); // 5MB in bytes (binary = 5,242,880 bytes)
    /// ```
    pub fn with_default_limit(mut self, limit: impl Into<SizeLimit>) -> Self {
        self.default_limit = limit.into().0;
        self
    }

    /// Builder method to set a size limit for a specific MIME type.
    ///
    /// This limit applies only when the content type exactly matches
    /// the provided MIME type (case-insensitive).
    ///
    /// # Arguments
    /// * `mime_type` - The exact MIME type to limit (e.g., "application/json")
    /// * `limit` - The size limit (human-readable string, `SizeLimit`, or bytes)
    ///
    /// # Returns
    /// `Self` for method chaining.
    ///
    /// # Examples
    /// ```
    /// use axum_jetpack::size_limit::SizeLimitConfig;
    ///
    /// let config = SizeLimitConfig::default()
    ///     .with_specific_limit("application/json", "100kb")  // JSON limited to 100,000 bytes
    ///     .with_specific_limit("application/xml", "500kb");  // XML limited to 500,000 bytes
    ///
    /// assert_eq!(config.get_limit_for_content_type("application/json"), 100_000);
    /// assert_eq!(config.get_limit_for_content_type("application/xml"), 500_000);
    /// ```
    pub fn with_specific_limit(mut self, mime_type: &str, limit: impl Into<SizeLimit>) -> Self {
        self.specific_limits.insert(mime_type.to_lowercase(), limit.into().0);
        self
    }

    /// Builder method to set a size limit for a wildcard MIME type pattern.
    ///
    /// This limit applies to all content types that match the wildcard pattern.
    /// Patterns must follow the format `"type/*"` where `type` is the MIME type
    /// (e.g., "image", "video", "application").
    ///
    /// # Arguments
    /// * `wildcard` - The wildcard pattern (e.g., "image/*", "application/*")
    /// * `limit` - The size limit (human-readable string, `SizeLimit`, or bytes)
    ///
    /// # Returns
    /// `Self` for method chaining.
    ///
    /// # Examples
    /// ```
    /// use axum_jetpack::size_limit::SizeLimitConfig;
    ///
    /// let config = SizeLimitConfig::default()
    ///     .with_wildcard_limit("image/*", "5mb")    // All images limited to 5,000,000 bytes
    ///     .with_wildcard_limit("video/*", "100mb"); // All videos limited to 100,000,000 bytes
    ///
    /// assert_eq!(config.get_limit_for_content_type("image/jpeg"), 5_000_000);
    /// assert_eq!(config.get_limit_for_content_type("image/png"), 5_000_000);
    /// assert_eq!(config.get_limit_for_content_type("video/mp4"), 100_000_000);
    /// ```
    pub fn with_wildcard_limit(mut self, wildcard: &str, limit: impl Into<SizeLimit>) -> Self {
        self.wildcard_limits.insert(wildcard.to_lowercase(), limit.into().0);
        self
    }

    /// Creates a new, empty `SizeLimitConfig`.
    ///
    /// This creates a configuration with default values:
    /// - `default_limit`: 1 megabyte (1,000,000 bytes)
    /// - Empty `specific_limits` and `wildcard_limits`
    ///
    /// # Examples
    /// ```
    /// use axum_jetpack::size_limit::SizeLimitConfig;
    ///
    /// let config = SizeLimitConfig::new();
    /// // Same as SizeLimitConfig::default()
    /// assert_eq!(config.default_limit, 1_000_000);
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Clears all specific limits from the configuration.
    ///
    /// # Examples
    /// ```
    /// use axum_jetpack::size_limit::SizeLimitConfig;
    ///
    /// let mut config = SizeLimitConfig::default()
    ///     .with_specific_limit("application/json", "100kb")
    ///     .with_specific_limit("text/plain", "50kb");
    ///
    /// config.clear_specific_limits();
    /// assert!(config.specific_limits.is_empty());
    /// ```
    pub fn clear_specific_limits(&mut self) {
        self.specific_limits.clear();
    }

    /// Clears all wildcard limits from the configuration.
    ///
    /// # Examples
    /// ```
    /// use axum_jetpack::size_limit::SizeLimitConfig;
    ///
    /// let mut config = SizeLimitConfig::default()
    ///     .with_wildcard_limit("image/*", "5mb")
    ///     .with_wildcard_limit("video/*", "10mb");
    ///
    /// config.clear_wildcard_limits();
    /// assert!(config.wildcard_limits.is_empty());
    /// ```
    pub fn clear_wildcard_limits(&mut self) {
        self.wildcard_limits.clear();
    }

    /// Clears all limits (specific, wildcard, and resets default to 1MB).
    ///
    /// # Examples
    /// ```
    /// use axum_jetpack::size_limit::SizeLimitConfig;
    ///
    /// let mut config = SizeLimitConfig::default()
    ///     .with_default_limit("10mb")
    ///     .with_specific_limit("application/json", "100kb")
    ///     .with_wildcard_limit("image/*", "5mb");
    ///
    /// config.clear_all_limits();
    /// assert_eq!(config.default_limit, 1_000_000); // Back to 1MB (1,000,000 bytes)
    /// assert!(config.specific_limits.is_empty());
    /// assert!(config.wildcard_limits.is_empty());
    /// ```
    pub fn clear_all_limits(&mut self) {
        self.default_limit = parse_human_size("1mb").unwrap_or(1_000_000);
        self.specific_limits.clear();
        self.wildcard_limits.clear();
    }
}

// Convenience implementation for easy construction
impl SizeLimitConfig {
    /// Creates a configuration with a custom default limit.
    ///
    /// This is a convenience alternative to `.with_default_limit()`.
    ///
    /// # Examples
    /// ```
    /// use axum_jetpack::size_limit::SizeLimitConfig;
    ///
    /// let config = SizeLimitConfig::with_default("50mb");
    /// // 50MB = 50,000,000 bytes
    /// assert_eq!(config.default_limit, 50_000_000);
    /// ```
    pub fn with_default(limit: impl Into<SizeLimit>) -> Self {
        Self::default().with_default_limit(limit)
    }
}