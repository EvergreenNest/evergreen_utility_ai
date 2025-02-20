use std::borrow::Cow;

use crate::{
    aggregator::{AggregationCtx, Aggregator},
    score::Score,
};

/// Creates an [`Aggregator`] that returns the average score of its children if
/// it is greater than or equal to the given threshold. If the average score is
/// less than the threshold, the aggregator returns [`Score::MIN`].
pub fn average(threshold: impl Into<Score>) -> impl Aggregator {
    AverageAggregator {
        threshold: threshold.into(),
    }
}

struct AverageAggregator {
    threshold: Score,
}

impl Aggregator for AverageAggregator {
    fn name(&self) -> Cow<'static, str> {
        Cow::Owned(format!("average({})", self.threshold))
    }

    fn aggregate(&mut self, ctx: AggregationCtx) -> Score {
        let len: usize = ctx.aggregation.scores.len();

        if len == 0 {
            return Score::MIN;
        }

        let sum: Score = ctx.aggregation.scores.into_iter().sum();
        let average = Score::new(sum.get() / len as f32);

        if average < self.threshold {
            Score::MIN
        } else {
            average
        }
    }
}
