
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeUnit {
    Bytes,
    Kilobytes,
    Megabytes,
    Gigabytes,
    Kibibytes,
    Mebibytes,
    Gibibytes,
    Kilobits,
    Megabits,
    Gigabits,
}

impl SizeUnit {

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "b" | "byte" | "bytes" => Some(SizeUnit::Bytes),
            "kb" | "kilobyte" | "kilobytes" => Some(SizeUnit::Kilobytes),
            "mb" | "megabyte" | "megabytes" => Some(SizeUnit::Megabytes),
            "gb" | "gigabyte" | "gigabytes" => Some(SizeUnit::Gigabytes),
            "kib" | "kibibyte" | "kibibytes" => Some(SizeUnit::Kibibytes),
            "mib" | "mebibyte" | "mebibytes" => Some(SizeUnit::Mebibytes),
            "gib" | "gibibyte" | "gibibytes" => Some(SizeUnit::Gibibytes),
            "kbit" | "kilobit" | "kilobits" => Some(SizeUnit::Kilobits),
            "mbit" | "megabit" | "megabits" => Some(SizeUnit::Megabits),
            "gbit" | "gigabit" | "gigabits" => Some(SizeUnit::Gigabits),
            _ => None,
        }
    }


    pub fn to_bytes(&self, value: f64) -> usize {
        match self {
            SizeUnit::Bytes => value as usize,
            SizeUnit::Kilobytes => (value * 1000.0) as usize,
            SizeUnit::Megabytes => (value * 1_000_000.0) as usize,
            SizeUnit::Gigabytes => (value * 1_000_000_000.0) as usize,
            SizeUnit::Kibibytes => (value * 1024.0) as usize,
            SizeUnit::Mebibytes => (value * 1_048_576.0) as usize,
            SizeUnit::Gibibytes => (value * 1_073_741_824.0) as usize,
            SizeUnit::Kilobits => (value * 125.0) as usize, // 1 kilobit = 125 bytes
            SizeUnit::Megabits => (value * 125_000.0) as usize,
            SizeUnit::Gigabits => (value * 125_000_000.0) as usize,
        }
    }
}

pub fn parse_human_size(size_str: &str) -> Result<usize, String> {
    let size_str = size_str.trim().to_lowercase();

    if size_str.is_empty() {
        return Err("Empty size string".to_string());
    }

    let mut num_end = 0;
    let chars: Vec<char> = size_str.chars().collect();

    for (i, c) in chars.iter().enumerate() {
        if c.is_digit(10) || *c == '.' || *c == ',' {
            num_end = i + 1;
        } else {
            break;
        }
    }

    if num_end == 0 {
        return Err("No number found".to_string());
    }


    let num_part = &size_str[..num_end];
    let num = num_part.replace(',', ".").parse::<f64>()
        .map_err(|e| format!("Invalid number '{}': {}", num_part, e))?;


    let unit_part = size_str[num_end..].trim();
    let unit = if unit_part.is_empty() {
        SizeUnit::Bytes
    } else {
        SizeUnit::from_str(unit_part)
            .ok_or_else(|| format!("Unknown unit '{}'", unit_part))?
    };

    Ok(unit.to_bytes(num))
}

#[derive(Debug, Clone, Copy)]
pub struct SizeLimit(pub usize);

impl From<usize> for SizeLimit {
    fn from(bytes: usize) -> Self {
        SizeLimit(bytes)
    }
}

impl From<&str> for SizeLimit {
    fn from(s: &str) -> Self {
        SizeLimit(parse_human_size(s).unwrap_or_else(|e| {
            panic!("Invalid size string '{}': {}", s, e)
        }))
    }
}

impl From<String> for SizeLimit {
    fn from(s: String) -> Self {
        s.as_str().into()
    }
}

impl SizeLimit {
    pub const KB: SizeLimit = SizeLimit(1024);
    pub const MB: SizeLimit = SizeLimit(1024 * 1024);
    pub const GB: SizeLimit = SizeLimit(1024 * 1024 * 1024);

    pub const KIB: SizeLimit = SizeLimit(1024);
    pub const MIB: SizeLimit = SizeLimit(1024 * 1024);
    pub const GIB: SizeLimit = SizeLimit(1024 * 1024 * 1024);

    pub fn bytes(bytes: usize) -> Self {
        SizeLimit(bytes)
    }

    pub fn kb(kb: f64) -> Self {
        SizeLimit((kb * 1000.0) as usize)
    }

    pub fn mb(mb: f64) -> Self {
        SizeLimit((mb * 1_000_000.0) as usize)
    }

    pub fn gb(gb: f64) -> Self {
        SizeLimit((gb * 1_000_000_000.0) as usize)
    }

    pub fn kib(kib: f64) -> Self {
        SizeLimit((kib * 1024.0) as usize)
    }

    pub fn mib(mib: f64) -> Self {
        SizeLimit((mib * 1_048_576.0) as usize)
    }

    pub fn gib(gib: f64) -> Self {
        SizeLimit((gib * 1_073_741_824.0) as usize)
    }

    pub fn kbit(kbit: f64) -> Self {
        SizeLimit((kbit * 125.0) as usize)
    }

    pub fn mbit(mbit: f64) -> Self {
        SizeLimit((mbit * 125_000.0) as usize)
    }

    pub fn gbit(gbit: f64) -> Self {
        SizeLimit((gbit * 125_000_000.0) as usize)
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_human_size() {
        assert_eq!(parse_human_size("1024").unwrap(), 1024);
        assert_eq!(parse_human_size("1KB").unwrap(), 1000);
        assert_eq!(parse_human_size("1kb").unwrap(), 1000);
        assert_eq!(parse_human_size("1.5MB").unwrap(), 1_500_000);
        assert_eq!(parse_human_size("2.5 GB").unwrap(), 2_500_000_000);
        assert_eq!(parse_human_size("1KiB").unwrap(), 1024);
        assert_eq!(parse_human_size("1MiB").unwrap(), 1_048_576);
        assert_eq!(parse_human_size("1GiB").unwrap(), 1_073_741_824);
        assert_eq!(parse_human_size("1Mbit").unwrap(), 125_000);
        assert_eq!(parse_human_size("10Mbit").unwrap(), 1_250_000);
        assert_eq!(parse_human_size("1Gbit").unwrap(), 125_000_000);

        assert_eq!(parse_human_size("1,5MB").unwrap(), 1_500_000);

        assert!(parse_human_size("").is_err());
        assert!(parse_human_size("abc").is_err());
        assert!(parse_human_size("1XB").is_err()); // Unknown unit
    }

    #[test]
    fn test_size_limit_from_str() {
        let limit: SizeLimit = "2MB".into();
        assert_eq!(limit.0, 2_000_000);

        let limit: SizeLimit = "1.5GB".into();
        assert_eq!(limit.0, 1_500_000_000);

        let limit: SizeLimit = "100Mbit".into();
        assert_eq!(limit.0, 12_500_000);
    }
}