use crate::error::PolicyError;

/// Bitcoin blocks per year (365.25 days * 24 * 60 / 10 minutes).
pub const BLOCKS_PER_YEAR: u32 = 52_560;

pub fn blocks_per_year() -> u32 {
    BLOCKS_PER_YEAR
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurationUnit {
    Blocks,
    Years,
}

/// Parse duration strings like `4y`, `210240b`, or plain block counts.
pub fn parse_duration(input: &str) -> Result<u32, PolicyError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(PolicyError::InvalidDuration("empty duration".into()));
    }

    if let Some(years) = input.strip_suffix('y') {
        let years: u32 = years
            .parse()
            .map_err(|_| PolicyError::InvalidDuration(format!("invalid years: {input}")))?;
        return years
            .checked_mul(BLOCKS_PER_YEAR)
            .ok_or_else(|| PolicyError::InvalidDuration(format!("duration overflow: {input}")));
    }

    if let Some(blocks) = input.strip_suffix('b') {
        return blocks
            .parse()
            .map_err(|_| PolicyError::InvalidDuration(format!("invalid blocks: {input}")));
    }

    input
        .parse()
        .map_err(|_| PolicyError::InvalidDuration(format!("invalid duration: {input}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_years_in_blocks() {
        assert_eq!(parse_duration("4y").unwrap(), 4 * BLOCKS_PER_YEAR);
    }

    #[test]
    fn plain_blocks() {
        assert_eq!(parse_duration("1008").unwrap(), 1008);
        assert_eq!(parse_duration("1008b").unwrap(), 1008);
    }
}
