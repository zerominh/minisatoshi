use crate::error::PolicyError;

/// ~10-minute blocks: 24 h × 6 blocks/hour.
pub const BLOCKS_PER_DAY: u32 = 144;

/// 7 × [`BLOCKS_PER_DAY`].
pub const BLOCKS_PER_WEEK: u32 = 1_008;

/// Bitcoin blocks per year (365.25 days × [`BLOCKS_PER_DAY`]).
pub const BLOCKS_PER_YEAR: u32 = 52_560;

pub fn blocks_per_day() -> u32 {
    BLOCKS_PER_DAY
}

pub fn blocks_per_week() -> u32 {
    BLOCKS_PER_WEEK
}

pub fn blocks_per_year() -> u32 {
    BLOCKS_PER_YEAR
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurationUnit {
    Blocks,
    Days,
    Weeks,
    Years,
}

/// Parse duration strings like `4y`, `2w`, `1d`, `210240b`, or plain block counts.
pub fn parse_duration(input: &str) -> Result<u32, PolicyError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(PolicyError::InvalidDuration("empty duration".into()));
    }

    // Multi-char suffixes first so `w`/`d`/`y`/`b` do not steal digits incorrectly.
    if let Some(rest) = input.strip_suffix('y') {
        return mul_unit(rest, BLOCKS_PER_YEAR, input, "years");
    }
    if let Some(rest) = input.strip_suffix('w') {
        return mul_unit(rest, BLOCKS_PER_WEEK, input, "weeks");
    }
    if let Some(rest) = input.strip_suffix('d') {
        return mul_unit(rest, BLOCKS_PER_DAY, input, "days");
    }
    if let Some(rest) = input.strip_suffix('b') {
        return rest
            .parse()
            .map_err(|_| PolicyError::InvalidDuration(format!("invalid blocks: {input}")));
    }

    input
        .parse()
        .map_err(|_| PolicyError::InvalidDuration(format!("invalid duration: {input}")))
}

fn mul_unit(raw: &str, per_unit: u32, original: &str, label: &str) -> Result<u32, PolicyError> {
    let count: u32 = raw
        .parse()
        .map_err(|_| PolicyError::InvalidDuration(format!("invalid {label}: {original}")))?;
    count
        .checked_mul(per_unit)
        .ok_or_else(|| PolicyError::InvalidDuration(format!("duration overflow: {original}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_years_in_blocks() {
        assert_eq!(parse_duration("4y").unwrap(), 4 * BLOCKS_PER_YEAR);
    }

    #[test]
    fn days_and_weeks_in_blocks() {
        assert_eq!(parse_duration("1d").unwrap(), BLOCKS_PER_DAY);
        assert_eq!(parse_duration("2d").unwrap(), 2 * BLOCKS_PER_DAY);
        assert_eq!(parse_duration("1w").unwrap(), BLOCKS_PER_WEEK);
        assert_eq!(parse_duration("2w").unwrap(), 2 * BLOCKS_PER_WEEK);
    }

    #[test]
    fn week_equals_seven_days() {
        assert_eq!(
            parse_duration("1w").unwrap(),
            parse_duration("7d").unwrap()
        );
    }

    #[test]
    fn plain_blocks() {
        assert_eq!(parse_duration("1008").unwrap(), 1008);
        assert_eq!(parse_duration("1008b").unwrap(), 1008);
    }

    #[test]
    fn rejects_empty_and_bad_suffix() {
        assert!(parse_duration("").is_err());
        assert!(parse_duration("3x").is_err());
        assert!(parse_duration("d").is_err());
    }
}
