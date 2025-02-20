use std::borrow::Cow;

use crate::{
    aggregator::{AggregationCtx, Aggregator},
    score::Score,
};

pub(crate) struct DifferenceAggregator;

impl Aggregator for DifferenceAggregator {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("difference")
    }

    fn aggregate(&mut self, ctx: AggregationCtx) -> Score {
        let [a, b] = [ctx.aggregation.scores[0], ctx.aggregation.scores[1]];
        a - b
    }
}
