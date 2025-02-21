use std::borrow::Cow;

use crate::{
    aggregator::{AggregationCtx, Aggregator},
    score::Score,
};

/// Creates an [`Aggregator`] that returns the average score of its children.
/// If no child scores are provided, [`Score::MIN`] is returned.
pub fn average() -> impl Aggregator {
    AverageAggregator
}

struct AverageAggregator;

impl Aggregator for AverageAggregator {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("average")
    }

    fn aggregate(&mut self, ctx: AggregationCtx) -> Score {
        let len: usize = ctx.aggregation.scores.len();

        if len == 0 {
            return Score::MIN;
        }

        let sum: Score = ctx.aggregation.scores.into_iter().sum();
        Score::new(sum.get() / len as f32)
    }
}
