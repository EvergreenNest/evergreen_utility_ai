//! Provides the [`Score`] type for representing a score value in the range `[0, 1]`,
//! and the [`Scoreable`] trait for converting values into scores.

use std::{
    cmp::Ordering,
    fmt,
    ops::{Add, Div, Mul, Sub},
};

use bevy_math::curve::Interval;

/// A score value in the range [0, 1]. Cannot be NaN.
#[derive(Clone, Copy, Debug, Default)]
pub struct Score {
    value: f32,
}

impl Score {
    /// The minimum possible score.
    pub const MIN: Score = Self { value: 0. };

    /// The maximum possible score.
    pub const MAX: Score = Self { value: 1. };

    /// The [`Interval`] of possible scores.
    pub const INTERVAL: Interval = Interval::UNIT;

    /// Creates a new score value.
    pub const fn new(value: f32) -> Self {
        if value.is_nan() {
            panic!("Score value must not be NaN");
        }
        Self {
            value: value.clamp(Self::MIN.get(), Self::MAX.get()),
        }
    }

    /// Returns the score value.
    #[inline(always)]
    pub const fn get(&self) -> f32 {
        self.value
    }

    /// Sets the score value.
    #[inline]
    pub const fn set(&mut self, value: f32) {
        if value.is_nan() {
            panic!("Score value must not be NaN");
        }
        self.value = value.clamp(Self::MIN.get(), Self::MAX.get());
    }
}

impl From<f32> for Score {
    fn from(value: f32) -> Self {
        Self::new(value)
    }
}

impl PartialEq for Score {
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl PartialEq<f32> for Score {
    fn eq(&self, other: &f32) -> bool {
        self.get() == *other
    }
}

impl PartialEq<Score> for f32 {
    fn eq(&self, other: &Score) -> bool {
        *self == other.get()
    }
}

impl Eq for Score {}

impl PartialOrd for Score {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialOrd<f32> for Score {
    fn partial_cmp(&self, other: &f32) -> Option<Ordering> {
        self.get().partial_cmp(other)
    }
}

impl PartialOrd<Score> for f32 {
    fn partial_cmp(&self, other: &Score) -> Option<Ordering> {
        self.partial_cmp(&other.get())
    }
}

impl Ord for Score {
    fn cmp(&self, other: &Self) -> Ordering {
        self.get()
            .partial_cmp(&other.get())
            .unwrap_or(Ordering::Equal)
    }
}

impl Add for Score {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.get() + rhs.get())
    }
}

impl Add<f32> for Score {
    type Output = Self;

    fn add(self, rhs: f32) -> Self::Output {
        Self::new(self.get() + rhs)
    }
}

impl Sub for Score {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.get() - rhs.get())
    }
}

impl Sub<f32> for Score {
    type Output = Self;

    fn sub(self, rhs: f32) -> Self::Output {
        Self::new(self.get() - rhs)
    }
}

impl Mul for Score {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(self.get() * rhs.get())
    }
}

impl Mul<f32> for Score {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.get() * rhs)
    }
}

impl Div for Score {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        if rhs.get() == 0. {
            Self::MIN
        } else {
            Self::new(self.get() / rhs.get())
        }
    }
}

impl Div<f32> for Score {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        if rhs == 0. {
            Self::MIN
        } else {
            Self::new(self.get() / rhs)
        }
    }
}

impl std::iter::Sum for Score {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::MIN, Add::add)
    }
}

impl std::iter::Product for Score {
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::MAX, Mul::mul)
    }
}

impl fmt::Display for Score {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.4}", self.get())
    }
}

/// Trait for types that can be converted into a [`Score`].
pub trait Scoreable {
    /// Convert the value into a [`Score`].
    fn score(&self) -> Score;
}

impl Scoreable for Score {
    #[inline(always)]
    fn score(&self) -> Score {
        *self
    }
}

impl<S: Scoreable> Scoreable for &S {
    #[inline(always)]
    fn score(&self) -> Score {
        (*self).score()
    }
}
