use std::borrow::Cow;

use crate::{
    aggregator::{AggregationCtx, Aggregator},
    score::Score,
};

/// Creates an [`Aggregator`] that returns the lowest score of its children if
/// it is greater than or equal to the given threshold. If the lowest score is
/// less than the threshold, the aggregator returns [`Score::MIN`].
pub fn minimum(threshold: impl Into<Score>) -> impl Aggregator {
    MinimumAggregator {
        threshold: threshold.into(),
    }
}

struct MinimumAggregator {
    threshold: Score,
}

impl Aggregator for MinimumAggregator {
    fn name(&self) -> Cow<'static, str> {
        Cow::Owned(format!("minimum({})", self.threshold))
    }

    fn aggregate(&mut self, ctx: AggregationCtx) -> Score {
        let min = ctx
            .aggregation
            .scores
            .into_iter()
            .min()
            .unwrap_or(Score::MIN);

        if min < self.threshold {
            Score::MIN
        } else {
            min
        }
    }
}
