use alloc::borrow::Cow;

use bevy_ecs::{
    component::Tick,
    system::{IntoSystem, ReadOnlySystem},
    world::World,
};

use crate::{
    label::InternedActionLabel,
    selector::{IntoSelector, Selection, SelectionCtx, Selector},
};

#[doc(hidden)]
pub struct SystemSelector<S> {
    system: S,
}

impl<'s, S: ReadOnlySystem<In = Selection<'s>, Out = Option<InternedActionLabel>>> Selector
    for SystemSelector<S>
{
    fn name(&self) -> Cow<'static, str> {
        self.system.name()
    }

    fn initialize(&mut self, world: &mut World) {
        self.system.initialize(world);
    }

    fn select(&mut self, ctx: SelectionCtx) -> Option<InternedActionLabel> {
        self.system.run_readonly(ctx.selection, ctx.world)
    }

    fn check_change_tick(&mut self, change_tick: Tick) {
        self.system.check_change_tick(change_tick);
    }
}

#[doc(hidden)]
pub struct SelectorSystemMarker;

impl<M, S> IntoSelector<(SelectorSystemMarker, M)> for S
where
    S: IntoSystem<Selection<'static>, Option<InternedActionLabel>, M, System: ReadOnlySystem>,
{
    type Selector = SystemSelector<S::System>;

    fn into_selector(self) -> Self::Selector {
        SystemSelector {
            system: IntoSystem::into_system(self),
        }
    }
}
