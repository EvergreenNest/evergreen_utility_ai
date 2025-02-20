use std::borrow::Cow;

use crate::{
    aggregator::{AggregationCtx, Aggregator},
    score::Score,
};

/// Creates an [`Aggregator`] that returns the median score of its children if
/// it is greater than or equal to the given threshold. If the median score is
/// less than the threshold, the aggregator returns [`Score::MIN`].
pub fn median(threshold: impl Into<Score>) -> impl Aggregator {
    MedianAggregator {
        threshold: threshold.into(),
    }
}

struct MedianAggregator {
    threshold: Score,
}

impl Aggregator for MedianAggregator {
    fn name(&self) -> Cow<'static, str> {
        Cow::Owned(format!("median({})", self.threshold))
    }

    fn aggregate(&mut self, ctx: AggregationCtx) -> Score {
        let mut scores = ctx.aggregation.scores.clone();
        scores.sort_unstable();

        let len = scores.len();
        if len == 0 {
            return Score::MIN;
        }

        let median = if len % 2 == 0 {
            let mid = len / 2;
            let left = scores[mid - 1];
            let right = scores[mid];
            Score::new((left.get() + right.get()) / 2.0)
        } else {
            scores[len / 2]
        };

        if median < self.threshold {
            Score::MIN
        } else {
            median
        }
    }
}
