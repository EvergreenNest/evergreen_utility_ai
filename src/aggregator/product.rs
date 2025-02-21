use std::borrow::Cow;

use crate::{
    aggregator::{AggregationCtx, Aggregator},
    score::Score,
};

/// Creates an [`Aggregator`] that multiplies the scores of its children.
/// If no child scores are provided, [`Score::MIN`] is returned.
pub fn product() -> impl Aggregator {
    ProductAggregator
}

struct ProductAggregator;

impl Aggregator for ProductAggregator {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("product")
    }

    fn aggregate(&mut self, ctx: AggregationCtx) -> Score {
        if ctx.aggregation.scores.is_empty() {
            return Score::MIN;
        }
        ctx.aggregation.scores.into_iter().product()
    }
}
