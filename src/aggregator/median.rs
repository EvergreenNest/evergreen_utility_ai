use alloc::borrow::Cow;

use crate::{
    aggregator::{AggregationCtx, Aggregator},
    score::Score,
};

/// Creates an [`Aggregator`] that returns the median score of its children.
/// If no child scores are provided, [`Score::MIN`] is returned.
pub fn median() -> impl Aggregator {
    MedianAggregator
}

struct MedianAggregator;

impl Aggregator for MedianAggregator {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("median")
    }

    fn aggregate(&mut self, ctx: AggregationCtx) -> Score {
        let mut scores = ctx.aggregation.scores;
        scores.sort_unstable();

        let len = scores.len();
        if len == 0 {
            return Score::MIN;
        }

        if len % 2 == 0 {
            let mid = len / 2;
            let left = scores[mid - 1];
            let right = scores[mid];
            Score::new((left.get() + right.get()) / 2.0)
        } else {
            scores[len / 2]
        }
    }
}
