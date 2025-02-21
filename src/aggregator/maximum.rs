use std::borrow::Cow;

use crate::{
    aggregator::{AggregationCtx, Aggregator},
    score::Score,
};

/// Creates an [`Aggregator`] that returns the highest score of its children.
/// If no child scores are provided, [`Score::MIN`] is returned.
pub fn maximum() -> impl Aggregator {
    MaximumAggregator
}

struct MaximumAggregator;

impl Aggregator for MaximumAggregator {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("maximum")
    }

    fn aggregate(&mut self, ctx: AggregationCtx) -> Score {
        ctx.aggregation
            .scores
            .into_iter()
            .max()
            .unwrap_or(Score::MIN)
    }
}
