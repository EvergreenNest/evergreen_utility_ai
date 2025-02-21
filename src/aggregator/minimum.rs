use alloc::borrow::Cow;

use crate::{
    aggregator::{AggregationCtx, Aggregator},
    score::Score,
};

/// Creates an [`Aggregator`] that returns the lowest score of its children.
/// If no child scores are provided, [`Score::MIN`] is returned.
pub fn minimum() -> impl Aggregator {
    MinimumAggregator
}

struct MinimumAggregator;

impl Aggregator for MinimumAggregator {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("minimum")
    }

    fn aggregate(&mut self, ctx: AggregationCtx) -> Score {
        ctx.aggregation
            .scores
            .into_iter()
            .min()
            .unwrap_or(Score::MIN)
    }
}
