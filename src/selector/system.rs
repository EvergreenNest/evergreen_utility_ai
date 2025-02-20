use std::borrow::Cow;

use bevy_ecs::{
    system::{IntoSystem, ReadOnlySystem},
    world::World,
};

use crate::{
    label::InternedActionLabel,
    selector::{IntoSelector, Selection, SelectionCtx, Selector},
};

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
