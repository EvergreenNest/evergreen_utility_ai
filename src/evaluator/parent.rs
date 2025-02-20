use std::{borrow::Cow, marker::PhantomData};

use bevy_ecs::component::Component;
use bevy_hierarchy::Parent;

use crate::{
    evaluator::{EvaluationCtx, Evaluator},
    score::{Score, Scoreable},
};

/// Creates a [`Evaluator`] that scores the given [`Component`] on the parent
/// entity of the target entity. If the target entity does not have a parent,
/// the evaluator returns [`Score::MIN`].
pub fn parent<C: Component + Scoreable>() -> impl Evaluator {
    ParentEvaluator(PhantomData::<C>)
}

struct ParentEvaluator<C: Component + Scoreable>(PhantomData<C>);

impl<C: Component + Scoreable> Evaluator for ParentEvaluator<C> {
    fn name(&self) -> Cow<'static, str> {
        Cow::Owned(format!("parent({})", std::any::type_name::<C>()))
    }

    fn evaluate(&mut self, ctx: EvaluationCtx) -> Score {
        let Some(parent) = ctx.world.get::<Parent>(ctx.evaluation.target) else {
            return Score::MIN;
        };

        ctx.world
            .get::<C>(parent.get())
            .map(|c| c.score())
            .unwrap_or(Score::MIN)
    }
}
