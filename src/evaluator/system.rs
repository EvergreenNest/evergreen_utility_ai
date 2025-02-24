use alloc::borrow::Cow;

use bevy_ecs::{
    component::Tick,
    system::{IntoSystem, ReadOnlySystem},
    world::World,
};

use crate::{
    evaluator::{Evaluation, EvaluationCtx, Evaluator, IntoEvaluator},
    score::Score,
};

#[doc(hidden)]
pub struct SystemEvaluator<S> {
    system: S,
}

impl<S> Evaluator for SystemEvaluator<S>
where
    S: ReadOnlySystem<In = Evaluation, Out = Score>,
{
    fn name(&self) -> Cow<'static, str> {
        self.system.name()
    }

    fn initialize(&mut self, world: &mut World) {
        self.system.initialize(world);
    }

    fn evaluate(&mut self, ctx: EvaluationCtx) -> Score {
        self.system.run_readonly(ctx.evaluation, ctx.world)
    }

    fn check_change_tick(&mut self, change_tick: Tick) {
        self.system.check_change_tick(change_tick);
    }
}

#[doc(hidden)]
pub struct EvaluatorSystemMarker;

impl<M, S> IntoEvaluator<(EvaluatorSystemMarker, M)> for S
where
    S: IntoSystem<Evaluation, Score, M, System: ReadOnlySystem>,
{
    type Evaluator = SystemEvaluator<S::System>;

    fn into_evaluator(self) -> Self::Evaluator {
        SystemEvaluator {
            system: IntoSystem::into_system(self),
        }
    }
}
