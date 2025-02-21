use std::borrow::Cow;

use crate::{
    aggregator::{AggregationCtx, Aggregator},
    score::Score,
};

/// Creates an [`Aggregator`] that returns the average score of its children.
/// If no child scores are provided, [`Score::MIN`] is returned.
#[doc(alias = "mean")]
pub fn average() -> impl Aggregator {
    AverageAggregator
}

struct AverageAggregator;

impl Aggregator for AverageAggregator {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("average")
    }

    fn aggregate(&mut self, ctx: AggregationCtx) -> Score {
        let len: usize = ctx.aggregation.scores.len();

        if len == 0 {
            return Score::MIN;
        }

        let sum: Score = ctx.aggregation.scores.into_iter().sum();
        Score::new(sum.get() / len as f32)
    }
}

/// Creates an [`Aggregator`] that returns the geometric mean of its children.
/// If no child scores are provided, [`Score::MIN`] is returned.
pub fn geometric_mean() -> impl Aggregator {
    GeometricMeanAggregator
}

struct GeometricMeanAggregator;

impl Aggregator for GeometricMeanAggregator {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("geometric_mean")
    }

    fn aggregate(&mut self, ctx: AggregationCtx) -> Score {
        let scores = ctx.aggregation.scores;

        let len = scores.len();
        if len == 0 {
            return Score::MIN;
        }

        let product = scores.iter().fold(1.0, |acc, score| acc * score.get());
        Score::new(product.powf(1.0 / len as f32))
    }
}

/// Creates an [`Aggregator`] that returns the harmonic mean of its children.
/// If no child scores are provided, [`Score::MIN`] is returned.
pub fn harmonic_mean() -> impl Aggregator {
    HarmonicMeanAggregator
}

struct HarmonicMeanAggregator;

impl Aggregator for HarmonicMeanAggregator {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("harmonic_mean")
    }

    fn aggregate(&mut self, ctx: AggregationCtx) -> Score {
        let scores = ctx.aggregation.scores;

        let len = scores.len();
        if len == 0 {
            return Score::MIN;
        }

        let sum = scores
            .iter()
            .fold(0.0, |acc, score| acc + 1.0 / score.get());
        Score::new(len as f32 / sum)
    }
}
