use std::borrow::Cow;

use crate::{
    evaluator::{EvaluationCtx, Evaluator},
    score::Score,
};

/// Creates a [`Evaluator`] that always returns the given [`Score`].
pub fn constant(score: impl Into<Score>) -> impl Evaluator {
    ConstantScoreEvaluator(score.into())
}

struct ConstantScoreEvaluator(Score);

impl Evaluator for ConstantScoreEvaluator {
    fn name(&self) -> Cow<'static, str> {
        Cow::Owned(format!("constant({})", self.0))
    }

    fn evaluate(&mut self, _ctx: EvaluationCtx) -> Score {
        self.0
    }
}
