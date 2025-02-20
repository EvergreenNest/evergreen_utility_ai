use std::borrow::Cow;

use crate::{
    aggregator::{AggregationCtx, Aggregator},
    score::Score,
};

/// Creates an [`Aggregator`] that sums the scores of its children.
/// If the sum is less than the given threshold, the aggregator returns
/// [`Score::MIN`].
pub fn sum(threshold: impl Into<Score>) -> impl Aggregator {
    SumAggregator {
        threshold: threshold.into(),
    }
}

struct SumAggregator {
    threshold: Score,
}

impl Aggregator for SumAggregator {
    fn name(&self) -> Cow<'static, str> {
        Cow::Owned(format!("sum({})", self.threshold))
    }

    fn aggregate(&mut self, ctx: AggregationCtx) -> Score {
        let sum = ctx.aggregation.scores.into_iter().sum();

        if sum < self.threshold {
            Score::MIN
        } else {
            sum
        }
    }
}
