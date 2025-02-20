use std::borrow::Cow;

use crate::{
    aggregator::{AggregationCtx, Aggregator},
    score::Score,
};

/// Creates an [`Aggregator`] that returns the highest score of its children if
/// it is greater than or equal to the given threshold. If the highest score is
/// less than the threshold, the aggregator returns [`Score::MIN`].
pub fn maximum(threshold: impl Into<Score>) -> impl Aggregator {
    MaximumAggregator {
        threshold: threshold.into(),
    }
}

struct MaximumAggregator {
    threshold: Score,
}

impl Aggregator for MaximumAggregator {
    fn name(&self) -> Cow<'static, str> {
        Cow::Owned(format!("maximum({})", self.threshold))
    }

    fn aggregate(&mut self, ctx: AggregationCtx) -> Score {
        let max = ctx
            .aggregation
            .scores
            .into_iter()
            .max()
            .unwrap_or(Score::MIN);

        if max < self.threshold {
            Score::MIN
        } else {
            max
        }
    }
}
