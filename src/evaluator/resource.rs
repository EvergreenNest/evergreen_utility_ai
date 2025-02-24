use alloc::{borrow::Cow, format};
use core::marker::PhantomData;

use bevy_ecs::resource::Resource;

use crate::{
    evaluator::{EvaluationCtx, Evaluator},
    score::{Score, Scoreable},
};

/// Creates a [`Evaluator`] that scores the current value of the given
/// [`Resource`]. If the resource is not present in the world, the evaluator
/// returns [`Score::MIN`].
pub fn resource<R: Resource + Scoreable>() -> impl Evaluator {
    ResourceEvaluator::<R>(PhantomData)
}

struct ResourceEvaluator<R: Resource + Scoreable>(PhantomData<R>);

impl<R: Resource + Scoreable> Evaluator for ResourceEvaluator<R> {
    fn name(&self) -> Cow<'static, str> {
        Cow::Owned(format!("resource({})", core::any::type_name::<R>()))
    }

    fn evaluate(&mut self, ctx: EvaluationCtx) -> Score {
        ctx.world
            .get_resource::<R>()
            .map(|r| r.score())
            .unwrap_or(Score::MIN)
    }
}
