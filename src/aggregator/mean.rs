use std::borrow::Cow;

use crate::{
    aggregator::{AggregationCtx, Aggregator},
    score::Score,
};

/// Creates an [`Aggregator`] that returns the geometric mean of its children if
/// it is greater than or equal to the given threshold. If the geometric mean is
/// less than the threshold, the aggregator returns [`Score::MIN`].
pub fn geometric_mean(threshold: impl Into<Score>) -> impl Aggregator {
    GeometricMeanAggregator {
        threshold: threshold.into(),
    }
}

struct GeometricMeanAggregator {
    threshold: Score,
}

impl Aggregator for GeometricMeanAggregator {
    fn name(&self) -> Cow<'static, str> {
        Cow::Owned(format!("geometric_mean({})", self.threshold))
    }

    fn aggregate(&mut self, ctx: AggregationCtx) -> Score {
        let scores = ctx.aggregation.scores;

        let len = scores.len();
        if len == 0 {
            return Score::MIN;
        }

        let product = scores.iter().fold(1.0, |acc, score| acc * score.get());
        let geometric_mean = Score::new(product.powf(1.0 / len as f32));

        if geometric_mean < self.threshold {
            Score::MIN
        } else {
            geometric_mean
        }
    }
}

/// Creates an [`Aggregator`] that returns the harmonic mean of its children if
/// it is greater than or equal to the given threshold. If the harmonic mean is
/// less than the threshold, the aggregator returns [`Score::MIN`].
pub fn harmonic_mean(threshold: impl Into<Score>) -> impl Aggregator {
    HarmonicMeanAggregator {
        threshold: threshold.into(),
    }
}

struct HarmonicMeanAggregator {
    threshold: Score,
}

impl Aggregator for HarmonicMeanAggregator {
    fn name(&self) -> Cow<'static, str> {
        Cow::Owned(format!("harmonic_mean({})", self.threshold))
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
        let harmonic_mean = Score::new(len as f32 / sum);

        if harmonic_mean < self.threshold {
            Score::MIN
        } else {
            harmonic_mean
        }
    }
}
