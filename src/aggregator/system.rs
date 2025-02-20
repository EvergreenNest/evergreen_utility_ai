use std::borrow::Cow;

use bevy_ecs::{
    system::{IntoSystem, ReadOnlySystem},
    world::World,
};

use crate::{
    aggregator::{Aggregation, AggregationCtx, Aggregator, IntoAggregator},
    score::Score,
};

pub struct SystemAggregator<S> {
    system: S,
}

impl<S> Aggregator for SystemAggregator<S>
where
    S: ReadOnlySystem<In = Aggregation, Out = Score>,
{
    fn name(&self) -> Cow<'static, str> {
        self.system.name()
    }

    fn initialize(&mut self, world: &mut World) {
        self.system.initialize(world);
    }

    fn aggregate(&mut self, ctx: AggregationCtx) -> Score {
        self.system.run_readonly(ctx.aggregation, ctx.world)
    }
}

#[doc(hidden)]
pub struct AggregatorSystemMarker;

impl<M, S> IntoAggregator<(AggregatorSystemMarker, M)> for S
where
    S: IntoSystem<Aggregation, Score, M, System: ReadOnlySystem>,
{
    type Aggregator = SystemAggregator<S::System>;

    fn into_aggregator(self) -> Self::Aggregator {
        SystemAggregator {
            system: IntoSystem::into_system(self),
        }
    }
}
