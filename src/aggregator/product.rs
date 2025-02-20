use std::borrow::Cow;

use crate::{
    aggregator::{AggregationCtx, Aggregator},
    score::Score,
};

/// Creates an [`Aggregator`] that multiplies the scores of its children.
/// If the product is less than the given threshold, the aggregator returns
/// [`Score::MIN`].
pub fn product(threshold: impl Into<Score>) -> impl Aggregator {
    ProductAggregator {
        threshold: threshold.into(),
    }
}

struct ProductAggregator {
    threshold: Score,
}

impl Aggregator for ProductAggregator {
    fn name(&self) -> Cow<'static, str> {
        Cow::Owned(format!("product({})", self.threshold))
    }

    fn aggregate(&mut self, ctx: AggregationCtx) -> Score {
        let product = ctx.aggregation.scores.into_iter().product();

        if product < self.threshold {
            Score::MIN
        } else {
            product
        }
    }
}
