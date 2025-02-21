use std::borrow::Cow;

use crate::{
    aggregator::{AggregationCtx, Aggregator},
    score::Score,
};

/// Creates an [`Aggregator`] that sums the scores of its children.
/// If no child scores are provided, [`Score::MIN`] is returned.
pub fn sum() -> impl Aggregator {
    SumAggregator
}

struct SumAggregator;

impl Aggregator for SumAggregator {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("sum")
    }

    fn aggregate(&mut self, ctx: AggregationCtx) -> Score {
        if ctx.aggregation.scores.is_empty() {
            return Score::MIN;
        }
        ctx.aggregation.scores.into_iter().sum()
    }
}
