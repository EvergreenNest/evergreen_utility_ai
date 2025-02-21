use std::borrow::Cow;

use crate::{
    aggregator::{AggregationCtx, Aggregator},
    score::Score,
};

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
