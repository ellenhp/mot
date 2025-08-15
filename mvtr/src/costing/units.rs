use std::ops::{Add, Div, Mul};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Forward,
    Reverse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TravelledDistance(pub(super) u64);

impl TravelledDistance {
    pub fn mm(&self) -> u64 {
        self.0
    }

    pub fn zero() -> TravelledDistance {
        TravelledDistance(0)
    }
}

impl Add for TravelledDistance {
    type Output = TravelledDistance;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0.saturating_add(rhs.0))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ElapsedTime(pub(super) u64);

impl ElapsedTime {
    pub fn ms(&self) -> u64 {
        self.0
    }

    pub fn zero() -> ElapsedTime {
        ElapsedTime(0)
    }
}

impl Add for ElapsedTime {
    type Output = ElapsedTime;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0.saturating_add(rhs.0))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TravelSpeed {
    um_per_ms: u32,
}

impl TravelSpeed {
    pub fn from_mph(mph: f64) -> TravelSpeed {
        let meters_per_second = mph * 0.44704;
        TravelSpeed {
            um_per_ms: (meters_per_second * 1000.0) as u32,
        }
    }

    pub fn from_meters_per_second(meters_per_second: f64) -> TravelSpeed {
        TravelSpeed {
            um_per_ms: (meters_per_second * 1000.0) as u32,
        }
    }

    pub fn min(&self, other: &TravelSpeed) -> TravelSpeed {
        TravelSpeed {
            um_per_ms: self.um_per_ms.min(other.um_per_ms),
        }
    }
}

impl Div<TravelSpeed> for TravelledDistance {
    type Output = Option<ElapsedTime>;

    fn div(self, rhs: TravelSpeed) -> Option<ElapsedTime> {
        if rhs.um_per_ms == 0 {
            return None;
        }
        let (micrometers, overflow) = self.0.overflowing_mul(1000);
        if overflow {
            tracing::warn!(
                "Overflow while calculating travel time. This indicates a logic error or a way longer than the circumference of the earth."
            );
            return None;
        }
        let milliseconds = micrometers / rhs.um_per_ms as u64;
        Some(ElapsedTime(milliseconds))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PartsPerMillion(u32);

impl PartsPerMillion {
    pub fn of(ppm: u32) -> PartsPerMillion {
        PartsPerMillion(ppm)
    }
}

impl Default for PartsPerMillion {
    fn default() -> Self {
        Self(0)
    }
}

impl Add for PartsPerMillion {
    type Output = PartsPerMillion;

    fn add(self, rhs: Self) -> Self::Output {
        PartsPerMillion(self.0.saturating_add(rhs.0))
    }
}

impl Mul<PartsPerMillion> for ElapsedTime {
    type Output = ElapsedTime;

    fn mul(self, rhs: PartsPerMillion) -> Self::Output {
        let (elapsed_nanoseconds, overflow) = self.0.overflowing_mul(rhs.0.into());
        if overflow {
            tracing::warn!("Overflowed during fixed point PPM calculation");
            return ElapsedTime::zero();
        }
        ElapsedTime(elapsed_nanoseconds / 1_000_000)
    }
}
