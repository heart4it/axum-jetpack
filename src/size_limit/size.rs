/// Represents units for expressing data sizes.
///
/// This enum supports both decimal (metric) and binary (IEC) units,
/// as well as bit-based units for network bandwidth.
///
/// # Decimal (Metric) Units
/// - Based on powers of 10 (1,000)
/// - Commonly used for storage, networking, and data transfer
/// - Follows SI (International System of Units) conventions
/// - Examples: KB, MB, GB
///
/// # Binary (IEC) Units
/// - Based on powers of 2 (1,024)
/// - Traditionally used for memory and file sizes
/// - Officially defined by IEC 60027-2
/// - Examples: KiB, MiB, GiB
///
/// # Bit Units
/// - Based on bits rather than bytes
/// - Commonly used for network bandwidth (e.g., "100 Mbps")
/// - 1 byte = 8 bits
/// - Examples: kbit, Mbit, Gbit
///
/// # Examples
/// ```
/// use axum_jetpack::size_limit::SizeUnit;
///
/// // Decimal units
/// assert_eq!(SizeUnit::from_str("MB"), Some(SizeUnit::Megabytes));
/// assert_eq!(SizeUnit::from_str("gigabyte"), Some(SizeUnit::Gigabytes));
///
/// // Binary units
/// assert_eq!(SizeUnit::from_str("MiB"), Some(SizeUnit::Mebibytes));
/// assert_eq!(SizeUnit::from_str("gibibytes"), Some(SizeUnit::Gibibytes));
///
/// // Bit units
/// assert_eq!(SizeUnit::from_str("Mbit"), Some(SizeUnit::Megabits));
/// assert_eq!(SizeUnit::from_str("gigabits"), Some(SizeUnit::Gigabits));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeUnit {
    /// Bytes (base unit)
    /// - 1 byte = 8 bits
    /// - Symbol: `B`, `byte`, `bytes`
    Bytes,

    /// Kilobytes (decimal)
    /// - 1 kilobyte = 1,000 bytes
    /// - Symbol: `KB`, `kilobyte`, `kilobytes`
    Kilobytes,

    /// Megabytes (decimal)
    /// - 1 megabyte = 1,000,000 bytes
    /// - Symbol: `MB`, `megabyte`, `megabytes`
    Megabytes,

    /// Gigabytes (decimal)
    /// - 1 gigabyte = 1,000,000,000 bytes
    /// - Symbol: `GB`, `gigabyte`, `gigabytes`
    Gigabytes,

    /// Kibibytes (binary)
    /// - 1 kibibyte = 1,024 bytes
    /// - Symbol: `KiB`, `kibibyte`, `kibibytes`
    Kibibytes,

    /// Mebibytes (binary)
    /// - 1 mebibyte = 1,048,576 bytes (1,024 × 1,024)
    /// - Symbol: `MiB`, `mebibyte`, `mebibytes`
    Mebibytes,

    /// Gibibytes (binary)
    /// - 1 gibibyte = 1,073,741,824 bytes (1,024 × 1,024 × 1,024)
    /// - Symbol: `GiB`, `gibibyte`, `gibibytes`
    Gibibytes,

    /// Kilobits (bit-based)
    /// - 1 kilobit = 1,000 bits = 125 bytes
    /// - Symbol: `kbit`, `kilobit`, `kilobits`
    Kilobits,

    /// Megabits (bit-based)
    /// - 1 megabit = 1,000,000 bits = 125,000 bytes
    /// - Symbol: `Mbit`, `megabit`, `megabits`
    Megabits,

    /// Gigabits (bit-based)
    /// - 1 gigabit = 1,000,000,000 bits = 125,000,000 bytes
    /// - Symbol: `Gbit`, `gigabit`, `gigabits`
    Gigabits,
}

impl SizeUnit {
    /// Parses a string representation of a size unit.
    ///
    /// This function is case-insensitive and accepts various common
    /// abbreviations and full names for each unit.
    ///
    /// # Arguments
    /// * `s` - The string to parse (e.g., "MB", "megabyte", "MiB", "Mbit")
    ///
    /// # Returns
    /// - `Some(SizeUnit)` if the string represents a valid unit
    /// - `None` if the string doesn't match any known unit
    ///
    /// # Supported Formats
    ///
    /// | Unit Type | Short Forms | Long Forms |
    /// |-----------|-------------|------------|
    /// | Bytes | `"b"` | `"byte"`, `"bytes"` |
    /// | Kilobytes | `"kb"` | `"kilobyte"`, `"kilobytes"` |
    /// | Megabytes | `"mb"` | `"megabyte"`, `"megabytes"` |
    /// | Gigabytes | `"gb"` | `"gigabyte"`, `"gigabytes"` |
    /// | Kibibytes | `"kib"` | `"kibibyte"`, `"kibibytes"` |
    /// | Mebibytes | `"mib"` | `"mebibyte"`, `"mebibytes"` |
    /// | Gibibytes | `"gib"` | `"gibibyte"`, `"gibibytes"` |
    /// | Kilobits | `"kbit"` | `"kilobit"`, `"kilobits"` |
    /// | Megabits | `"mbit"` | `"megabit"`, `"megabits"` |
    /// | Gigabits | `"gbit"` | `"gigabit"`, `"gigabits"` |
    ///
    /// # Examples
    /// ```
    /// use axum_jetpack::size_limit::SizeUnit;
    ///
    /// // Case-insensitive parsing
    /// assert_eq!(SizeUnit::from_str("MB"), Some(SizeUnit::Megabytes));
    /// assert_eq!(SizeUnit::from_str("mb"), Some(SizeUnit::Megabytes));
    /// assert_eq!(SizeUnit::from_str("Mb"), Some(SizeUnit::Megabytes));
    ///
    /// // Full names work too
    /// assert_eq!(SizeUnit::from_str("megabyte"), Some(SizeUnit::Megabytes));
    /// assert_eq!(SizeUnit::from_str("megabytes"), Some(SizeUnit::Megabytes));
    ///
    /// // Binary units
    /// assert_eq!(SizeUnit::from_str("MiB"), Some(SizeUnit::Mebibytes));
    /// assert_eq!(SizeUnit::from_str("mebibyte"), Some(SizeUnit::Mebibytes));
    ///
    /// // Bit units
    /// assert_eq!(SizeUnit::from_str("Mbit"), Some(SizeUnit::Megabits));
    /// assert_eq!(SizeUnit::from_str("megabit"), Some(SizeUnit::Megabits));
    ///
    /// // Unknown units return None
    /// assert_eq!(SizeUnit::from_str("TB"), None); // Terabytes not supported
    /// assert_eq!(SizeUnit::from_str("foo"), None);
    /// ```
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            // Byte units
            "b" | "byte" | "bytes" => Some(SizeUnit::Bytes),

            // Decimal (metric) units
            "kb" | "kilobyte" | "kilobytes" => Some(SizeUnit::Kilobytes),
            "mb" | "megabyte" | "megabytes" => Some(SizeUnit::Megabytes),
            "gb" | "gigabyte" | "gigabytes" => Some(SizeUnit::Gigabytes),

            // Binary (IEC) units
            "kib" | "kibibyte" | "kibibytes" => Some(SizeUnit::Kibibytes),
            "mib" | "mebibyte" | "mebibytes" => Some(SizeUnit::Mebibytes),
            "gib" | "gibibyte" | "gibibytes" => Some(SizeUnit::Gibibytes),

            // Bit units
            "kbit" | "kilobit" | "kilobits" => Some(SizeUnit::Kilobits),
            "mbit" | "megabit" | "megabits" => Some(SizeUnit::Megabits),
            "gbit" | "gigabit" | "gigabits" => Some(SizeUnit::Gigabits),

            // Unknown unit
            _ => None,
        }
    }

    /// Converts a value in this unit to bytes.
    ///
    /// # Arguments
    /// * `value` - The numeric value to convert (e.g., 1.5 for "1.5MB")
    ///
    /// # Returns
    /// The equivalent number of bytes as `usize`.
    ///
    /// # Examples
    /// ```
    /// use axum_jetpack::size_limit::SizeUnit;
    ///
    /// // Decimal units
    /// assert_eq!(SizeUnit::Kilobytes.to_bytes(1.0), 1_000);
    /// assert_eq!(SizeUnit::Megabytes.to_bytes(2.5), 2_500_000);
    /// assert_eq!(SizeUnit::Gigabytes.to_bytes(0.5), 500_000_000);
    ///
    /// // Binary units
    /// assert_eq!(SizeUnit::Kibibytes.to_bytes(1.0), 1_024);
    /// assert_eq!(SizeUnit::Mebibytes.to_bytes(1.0), 1_048_576);
    /// assert_eq!(SizeUnit::Gibibytes.to_bytes(1.0), 1_073_741_824);
    ///
    /// // Bit units
    /// assert_eq!(SizeUnit::Kilobits.to_bytes(8.0), 1_000); // 8 kilobits = 1,000 bytes
    /// assert_eq!(SizeUnit::Megabits.to_bytes(1.0), 125_000);
    /// assert_eq!(SizeUnit::Gigabits.to_bytes(1.0), 125_000_000);
    ///
    /// // Bytes (no conversion needed)
    /// assert_eq!(SizeUnit::Bytes.to_bytes(1024.0), 1024);
    /// ```
    pub fn to_bytes(&self, value: f64) -> usize {
        match self {
            // Byte units (no conversion)
            SizeUnit::Bytes => value as usize,

            // Decimal units (powers of 10)
            SizeUnit::Kilobytes => (value * 1000.0) as usize,
            SizeUnit::Megabytes => (value * 1_000_000.0) as usize,
            SizeUnit::Gigabytes => (value * 1_000_000_000.0) as usize,

            // Binary units (powers of 2)
            SizeUnit::Kibibytes => (value * 1024.0) as usize,
            SizeUnit::Mebibytes => (value * 1_048_576.0) as usize,
            SizeUnit::Gibibytes => (value * 1_073_741_824.0) as usize,

            // Bit units (1 byte = 8 bits)
            SizeUnit::Kilobits => (value * 125.0) as usize,     // 1 kilobit = 125 bytes
            SizeUnit::Megabits => (value * 125_000.0) as usize, // 1 megabit = 125,000 bytes
            SizeUnit::Gigabits => (value * 125_000_000.0) as usize, // 1 gigabit = 125,000,000 bytes
        }
    }
}

/// Parses a human-readable size string into bytes.
///
/// This function supports strings like "1MB", "100kb", "2.5GB", "1.5 MiB", etc.
/// It handles both decimal and binary units, and accepts comma or period as
/// decimal separator.
///
/// # Arguments
/// * `size_str` - A human-readable size string (e.g., "1.5MB", "100kb", "2,5 GB")
///
/// # Returns
/// - `Ok(usize)` - The size in bytes if parsing succeeds
/// - `Err(String)` - An error message if parsing fails
///
/// # Supported Formats
/// - Numbers: Can be integers or decimals (e.g., "1024", "1.5", "2,5")
/// - Units: See `SizeUnit::from_str()` for supported units
/// - Whitespace: Optional space between number and unit (e.g., "1 MB" or "1MB")
/// - Case: Case-insensitive (e.g., "mb", "MB", "Mb" all work)
/// - Decimal separator: Both period (.) and comma (,) are accepted
///
/// # Examples
/// ```
/// use axum_jetpack::size_limit::parse_human_size;
///
/// // Decimal units
/// assert_eq!(parse_human_size("1KB").unwrap(), 1_000);
/// assert_eq!(parse_human_size("2.5MB").unwrap(), 2_500_000);
/// assert_eq!(parse_human_size("1 GB").unwrap(), 1_000_000_000);
///
/// // Binary units
/// assert_eq!(parse_human_size("1KiB").unwrap(), 1_024);
/// assert_eq!(parse_human_size("2 MiB").unwrap(), 2_097_152);
/// assert_eq!(parse_human_size("1.5GiB").unwrap(), 1_610_612_736);
///
/// // Bit units
/// assert_eq!(parse_human_size("1Mbit").unwrap(), 125_000);
/// assert_eq!(parse_human_size("10 Gbit").unwrap(), 1_250_000_000);
///
/// // Bytes (no unit specified)
/// assert_eq!(parse_human_size("1024").unwrap(), 1_024);
/// assert_eq!(parse_human_size("4096").unwrap(), 4_096);
///
/// // International decimal format (comma separator)
/// assert_eq!(parse_human_size("1,5MB").unwrap(), 1_500_000);
/// assert_eq!(parse_human_size("2,5 GB").unwrap(), 2_500_000_000);
///
/// // Error cases
/// assert!(parse_human_size("").is_err()); // Empty string
/// assert!(parse_human_size("abc").is_err()); // No number
/// assert!(parse_human_size("1XB").is_err()); // Unknown unit
/// assert!(parse_human_size("1.2.3MB").is_err()); // Invalid number
/// ```
pub fn parse_human_size(size_str: &str) -> Result<usize, String> {
    // Normalize input: trim whitespace and convert to lowercase
    let size_str = size_str.trim().to_lowercase();

    // Check for empty input
    if size_str.is_empty() {
        return Err("Empty size string".to_string());
    }

    // Find where the number part ends
    let mut num_end = 0;
    let chars: Vec<char> = size_str.chars().collect();

    for (i, c) in chars.iter().enumerate() {
        // Accept digits, period, or comma as part of the number
        if c.is_digit(10) || *c == '.' || *c == ',' {
            num_end = i + 1;
        } else {
            // Stop at first non-numeric character (start of unit)
            break;
        }
    }

    // Ensure we found a number
    if num_end == 0 {
        return Err("No number found".to_string());
    }

    // Extract and parse the number part
    let num_part = &size_str[..num_end];
    // Replace comma with period for consistent parsing
    let num = num_part.replace(',', ".").parse::<f64>()
        .map_err(|e| format!("Invalid number '{}': {}", num_part, e))?;

    // Extract and parse the unit part (if any)
    let unit_part = size_str[num_end..].trim();
    let unit = if unit_part.is_empty() {
        // Default to bytes if no unit specified
        SizeUnit::Bytes
    } else {
        SizeUnit::from_str(unit_part)
            .ok_or_else(|| format!("Unknown unit '{}'", unit_part))?
    };

    // Convert to bytes
    Ok(unit.to_bytes(num))
}

/// A type-safe wrapper for size limits in bytes.
///
/// This struct provides a convenient way to work with size limits
/// and supports conversion from various formats (strings, numbers).
///
/// # Examples
/// ```
/// use axum_jetpack::size_limit::SizeLimit;
///
/// // From string (human-readable format)
/// let limit1: SizeLimit = "2MB".into();
/// assert_eq!(limit1.0, 2_000_000);
///
/// // From number (bytes)
/// let limit2 = SizeLimit::from(1024);
/// assert_eq!(limit2.0, 1024);
///
/// // Using constructor methods
/// let limit3 = SizeLimit::mb(1.5);
/// assert_eq!(limit3.0, 1_500_000);
///
/// let limit4 = SizeLimit::mib(2.0);
/// assert_eq!(limit4.0, 2_097_152);
///
/// // Using constants (binary units)
/// assert_eq!(SizeLimit::KB.0, 1024);
/// assert_eq!(SizeLimit::MB.0, 1_048_576);
/// assert_eq!(SizeLimit::GB.0, 1_073_741_824);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct SizeLimit(pub usize);

impl From<usize> for SizeLimit {
    /// Creates a `SizeLimit` from a raw byte count.
    ///
    /// # Examples
    /// ```
    /// use axum_jetpack::size_limit::SizeLimit;
    ///
    /// let limit = SizeLimit::from(1024);
    /// assert_eq!(limit.0, 1024);
    /// ```
    fn from(bytes: usize) -> Self {
        SizeLimit(bytes)
    }
}

impl From<&str> for SizeLimit {
    /// Creates a `SizeLimit` from a human-readable string.
    ///
    /// # Panics
    /// Panics if the string cannot be parsed. Use `parse_human_size()`
    /// directly if you need error handling.
    ///
    /// # Examples
    /// ```
    /// use axum_jetpack::size_limit::SizeLimit;
    ///
    /// let limit: SizeLimit = "10MB".into();
    /// assert_eq!(limit.0, 10_000_000);
    /// ```
    fn from(s: &str) -> Self {
        SizeLimit(parse_human_size(s).unwrap_or_else(|e| {
            panic!("Invalid size string '{}': {}", s, e)
        }))
    }
}

impl From<String> for SizeLimit {
    /// Creates a `SizeLimit` from a `String`.
    ///
    /// # Panics
    /// Panics if the string cannot be parsed.
    ///
    /// # Examples
    /// ```
    /// use axum_jetpack::size_limit::SizeLimit;
    ///
    /// let s = String::from("100KB");
    /// let limit: SizeLimit = s.into();
    /// assert_eq!(limit.0, 100_000);
    /// ```
    fn from(s: String) -> Self {
        s.as_str().into()
    }
}

impl SizeLimit {
    /// Binary kilobyte constant (1,024 bytes).
    /// Note: This uses binary units despite the name "KB".
    pub const KB: SizeLimit = SizeLimit(1024);

    /// Binary megabyte constant (1,048,576 bytes).
    /// Note: This uses binary units despite the name "MB".
    pub const MB: SizeLimit = SizeLimit(1024 * 1024);

    /// Binary gigabyte constant (1,073,741,824 bytes).
    /// Note: This uses binary units despite the name "GB".
    pub const GB: SizeLimit = SizeLimit(1024 * 1024 * 1024);

    /// Kibibyte constant (1,024 bytes).
    pub const KIB: SizeLimit = SizeLimit(1024);

    /// Mebibyte constant (1,048,576 bytes).
    pub const MIB: SizeLimit = SizeLimit(1024 * 1024);

    /// Gibibyte constant (1,073,741,824 bytes).
    pub const GIB: SizeLimit = SizeLimit(1024 * 1024 * 1024);

    /// Creates a `SizeLimit` from a raw byte count.
    ///
    /// # Examples
    /// ```
    /// use axum_jetpack::size_limit::SizeLimit;
    ///
    /// let limit = SizeLimit::bytes(1024);
    /// assert_eq!(limit.0, 1024);
    /// ```
    pub fn bytes(bytes: usize) -> Self {
        SizeLimit(bytes)
    }

    /// Creates a `SizeLimit` from decimal kilobytes.
    ///
    /// # Arguments
    /// * `kb` - Number of kilobytes (1 KB = 1,000 bytes)
    ///
    /// # Examples
    /// ```
    /// use axum_jetpack::size_limit::SizeLimit;
    ///
    /// let limit = SizeLimit::kb(2.5);
    /// assert_eq!(limit.0, 2_500); // 2.5 × 1,000
    /// ```
    pub fn kb(kb: f64) -> Self {
        SizeLimit((kb * 1000.0) as usize)
    }

    /// Creates a `SizeLimit` from decimal megabytes.
    ///
    /// # Arguments
    /// * `mb` - Number of megabytes (1 MB = 1,000,000 bytes)
    ///
    /// # Examples
    /// ```
    /// use axum_jetpack::size_limit::SizeLimit;
    ///
    /// let limit = SizeLimit::mb(1.5);
    /// assert_eq!(limit.0, 1_500_000); // 1.5 × 1,000,000
    /// ```
    pub fn mb(mb: f64) -> Self {
        SizeLimit((mb * 1_000_000.0) as usize)
    }

    /// Creates a `SizeLimit` from decimal gigabytes.
    ///
    /// # Arguments
    /// * `gb` - Number of gigabytes (1 GB = 1,000,000,000 bytes)
    ///
    /// # Examples
    /// ```
    /// use axum_jetpack::size_limit::SizeLimit;
    ///
    /// let limit = SizeLimit::gb(0.5);
    /// assert_eq!(limit.0, 500_000_000); // 0.5 × 1,000,000,000
    /// ```
    pub fn gb(gb: f64) -> Self {
        SizeLimit((gb * 1_000_000_000.0) as usize)
    }

    /// Creates a `SizeLimit` from binary kibibytes.
    ///
    /// # Arguments
    /// * `kib` - Number of kibibytes (1 KiB = 1,024 bytes)
    ///
    /// # Examples
    /// ```
    /// use axum_jetpack::size_limit::SizeLimit;
    ///
    /// let limit = SizeLimit::kib(2.0);
    /// assert_eq!(limit.0, 2_048); // 2 × 1,024
    /// ```
    pub fn kib(kib: f64) -> Self {
        SizeLimit((kib * 1024.0) as usize)
    }

    /// Creates a `SizeLimit` from binary mebibytes.
    ///
    /// # Arguments
    /// * `mib` - Number of mebibytes (1 MiB = 1,048,576 bytes)
    ///
    /// # Examples
    /// ```
    /// use axum_jetpack::size_limit::SizeLimit;
    ///
    /// let limit = SizeLimit::mib(1.5);
    /// assert_eq!(limit.0, 1_572_864); // 1.5 × 1,048,576
    /// ```
    pub fn mib(mib: f64) -> Self {
        SizeLimit((mib * 1_048_576.0) as usize)
    }

    /// Creates a `SizeLimit` from binary gibibytes.
    ///
    /// # Arguments
    /// * `gib` - Number of gibibytes (1 GiB = 1,073,741,824 bytes)
    ///
    /// # Examples
    /// ```
    /// use axum_jetpack::size_limit::SizeLimit;
    ///
    /// let limit = SizeLimit::gib(0.25);
    /// assert_eq!(limit.0, 268_435_456); // 0.25 × 1,073,741,824
    /// ```
    pub fn gib(gib: f64) -> Self {
        SizeLimit((gib * 1_073_741_824.0) as usize)
    }

    /// Creates a `SizeLimit` from kilobits.
    ///
    /// # Arguments
    /// * `kbit` - Number of kilobits (1 kbit = 125 bytes)
    ///
    /// # Examples
    /// ```
    /// use axum_jetpack::size_limit::SizeLimit;
    ///
    /// let limit = SizeLimit::kbit(8.0);
    /// assert_eq!(limit.0, 1_000); // 8 × 125 = 1,000 bytes
    /// ```
    pub fn kbit(kbit: f64) -> Self {
        SizeLimit((kbit * 125.0) as usize)
    }

    /// Creates a `SizeLimit` from megabits.
    ///
    /// # Arguments
    /// * `mbit` - Number of megabits (1 Mbit = 125,000 bytes)
    ///
    /// # Examples
    /// ```
    /// use axum_jetpack::size_limit::SizeLimit;
    ///
    /// let limit = SizeLimit::mbit(10.0);
    /// assert_eq!(limit.0, 1_250_000); // 10 × 125,000
    /// ```
    pub fn mbit(mbit: f64) -> Self {
        SizeLimit((mbit * 125_000.0) as usize)
    }

    /// Creates a `SizeLimit` from gigabits.
    ///
    /// # Arguments
    /// * `gbit` - Number of gigabits (1 Gbit = 125,000,000 bytes)
    ///
    /// # Examples
    /// ```
    /// use axum_jetpack::size_limit::SizeLimit;
    ///
    /// let limit = SizeLimit::gbit(1.0);
    /// assert_eq!(limit.0, 125_000_000);
    /// ```
    pub fn gbit(gbit: f64) -> Self {
        SizeLimit((gbit * 125_000_000.0) as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_human_size() {
        // Test bytes (no unit)
        assert_eq!(parse_human_size("1024").unwrap(), 1024);

        // Test decimal units
        assert_eq!(parse_human_size("1KB").unwrap(), 1000);
        assert_eq!(parse_human_size("1kb").unwrap(), 1000); // Case insensitive
        assert_eq!(parse_human_size("1.5MB").unwrap(), 1_500_000);
        assert_eq!(parse_human_size("2.5 GB").unwrap(), 2_500_000_000); // With space

        // Test binary units
        assert_eq!(parse_human_size("1KiB").unwrap(), 1024);
        assert_eq!(parse_human_size("1MiB").unwrap(), 1_048_576);
        assert_eq!(parse_human_size("1GiB").unwrap(), 1_073_741_824);

        // Test bit units
        assert_eq!(parse_human_size("1Mbit").unwrap(), 125_000);
        assert_eq!(parse_human_size("10Mbit").unwrap(), 1_250_000);
        assert_eq!(parse_human_size("1Gbit").unwrap(), 125_000_000);

        // Test international decimal format (comma separator)
        assert_eq!(parse_human_size("1,5MB").unwrap(), 1_500_000);

        // Test error cases
        assert!(parse_human_size("").is_err()); // Empty string
        assert!(parse_human_size("abc").is_err()); // No number
        assert!(parse_human_size("1XB").is_err()); // Unknown unit
    }

    #[test]
    fn test_size_limit_from_str() {
        // Test From<&str> implementation
        let limit: SizeLimit = "2MB".into();
        assert_eq!(limit.0, 2_000_000);

        let limit: SizeLimit = "1.5GB".into();
        assert_eq!(limit.0, 1_500_000_000);

        let limit: SizeLimit = "100Mbit".into();
        assert_eq!(limit.0, 12_500_000); // 100 × 125,000
    }
}