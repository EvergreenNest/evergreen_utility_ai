use std::{borrow::Cow, marker::PhantomData};

use bevy_ecs::component::Component;

use crate::{
    evaluator::{EvaluationCtx, Evaluator},
    score::{Score, Scoreable},
};

/// Creates a [`Evaluator`] that scores the given [`Component`] on the target
/// entity. If the target entity does not have the component, the evaluator
/// returns [`Score::MIN`].
pub fn target<C: Component + Scoreable>() -> impl Evaluator {
    TargetEvaluator::<C>(PhantomData)
}

struct TargetEvaluator<C: Component + Scoreable>(PhantomData<C>);

impl<C: Component + Scoreable> Evaluator for TargetEvaluator<C> {
    fn name(&self) -> Cow<'static, str> {
        Cow::Owned(format!("target({})", std::any::type_name::<C>()))
    }

    fn evaluate(&mut self, ctx: EvaluationCtx) -> Score {
        ctx.world
            .get::<C>(ctx.evaluation.target)
            .map(|c| c.score())
            .unwrap_or(Score::MIN)
    }
}
