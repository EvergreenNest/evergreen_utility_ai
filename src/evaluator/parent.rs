use alloc::{borrow::Cow, format};
use core::marker::PhantomData;

use bevy_ecs::{component::Component, hierarchy::ChildOf};

use crate::{
    evaluator::{EvaluationCtx, Evaluator},
    score::{Score, Scoreable},
};

/// Creates a [`Evaluator`] that scores the given [`Component`] on the parent
/// entity of the target entity. If the target entity does not have a parent, or
/// if the parent entity does not have the given component, the evaluator
/// returns [`Score::MIN`].
pub fn parent<C: Component + Scoreable>() -> impl Evaluator {
    ParentEvaluator(PhantomData::<C>)
}

struct ParentEvaluator<C: Component + Scoreable>(PhantomData<C>);

impl<C: Component + Scoreable> Evaluator for ParentEvaluator<C> {
    fn name(&self) -> Cow<'static, str> {
        Cow::Owned(format!("parent({})", core::any::type_name::<C>()))
    }

    fn evaluate(&mut self, ctx: EvaluationCtx) -> Score {
        let Some(parent) = ctx.world.get::<ChildOf>(ctx.evaluation.target) else {
            return Score::MIN;
        };

        ctx.world
            .get::<C>(parent.get())
            .map(|c| c.score())
            .unwrap_or(Score::MIN)
    }
}
