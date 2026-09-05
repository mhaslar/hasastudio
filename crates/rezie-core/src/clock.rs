use serde::{Deserialize, Serialize};
use std::{num::NonZeroU32, time::Duration};

/// A positive rational programme rate, expressed as frames per second.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RawFrameRate")]
pub struct FrameRate {
    numerator: NonZeroU32,
    denominator: NonZeroU32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFrameRate {
    numerator: u32,
    denominator: u32,
}

impl TryFrom<RawFrameRate> for FrameRate {
    type Error = ClockError;
    fn try_from(value: RawFrameRate) -> Result<Self, Self::Error> {
        Self::new(value.numerator, value.denominator)
    }
}

/// Invalid timeline input or a timeline too long to represent.
#[derive(Debug, thiserror::Error)]
pub enum ClockError {
    /// A rational rate must have positive terms and at most 1000 fps.
    #[error("invalid frame rate {numerator}/{denominator}: terms must be positive and rate at most 1000 fps")]
    InvalidRate {
        /// Supplied numerator.
        numerator: u32,
        /// Supplied denominator.
        denominator: u32,
    },
    /// Duration cannot represent this timestamp.
    #[error("frame index {0} exceeds the representable programme timeline")]
    Overflow(u64),
}

impl FrameRate {
    /// Validate a rational rate without floating-point rounding.
    pub fn new(numerator: u32, denominator: u32) -> Result<Self, ClockError> {
        match (NonZeroU32::new(numerator), NonZeroU32::new(denominator)) {
            (Some(n), Some(d)) if u64::from(numerator) <= u64::from(denominator) * 1000 => {
                Ok(Self {
                    numerator: n,
                    denominator: d,
                })
            }
            _ => Err(ClockError::InvalidRate {
                numerator,
                denominator,
            }),
        }
    }

    /// Frames-per-second numerator.
    pub fn numerator(self) -> u32 {
        self.numerator.get()
    }
    /// Frames-per-second denominator.
    pub fn denominator(self) -> u32 {
        self.denominator.get()
    }

    /// Absolute presentation time, rounded down to nanoseconds only once.
    pub fn pts(self, index: u64) -> Result<Duration, ClockError> {
        let nanos = u128::from(index) * u128::from(self.denominator.get()) * 1_000_000_000
            / u128::from(self.numerator.get());
        let seconds =
            u64::try_from(nanos / 1_000_000_000).map_err(|_| ClockError::Overflow(index))?;
        Ok(Duration::new(seconds, (nanos % 1_000_000_000) as u32))
    }
}

impl Default for FrameRate {
    fn default() -> Self {
        Self {
            numerator: NonZeroU32::new(50).unwrap_or(NonZeroU32::MIN),
            denominator: NonZeroU32::MIN,
        }
    }
}

/// A payload-free tick on the common programme timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameTime {
    /// Absolute zero-based frame index.
    pub index: u64,
    /// Presentation time relative to the programme origin.
    pub pts: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute_timeline_does_not_accumulate_rounding() {
        assert_eq!(
            FrameRate::default().pts(30_000).unwrap(),
            Duration::from_secs(600)
        );
        let rate = FrameRate::new(60_000, 1001).unwrap();
        assert_eq!(rate.pts(60_000).unwrap(), Duration::from_secs(1001));
        assert!(rate.pts(1).unwrap() * 60_000 < rate.pts(60_000).unwrap());
    }

    #[test]
    fn invalid_rates_and_overflow_are_errors() {
        for (n, d) in [(0, 1), (1, 0), (1001, 1)] {
            assert!(FrameRate::new(n, d).is_err());
        }
        assert!(FrameRate::new(1, u32::MAX).unwrap().pts(u64::MAX).is_err());
        assert!(serde_json::from_str::<FrameRate>(r#"{"numerator":0,"denominator":1}"#).is_err());
        assert!(
            serde_json::from_str::<FrameRate>(r#"{"numerator":1001,"denominator":1}"#).is_err()
        );
    }
}
