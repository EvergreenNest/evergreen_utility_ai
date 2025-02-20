use std::borrow::Cow;

use bevy_ecs::{
    system::{IntoSystem, ReadOnlySystem},
    world::World,
};

use crate::mapper::{IntoMapper, Mapper, Mapping, MappingCtx};

/// A [`Mapper`] that calls a [`ReadOnlySystem`] to map values.
pub struct SystemMapper<S> {
    system: S,
}

impl<T, S> Mapper<T> for SystemMapper<S>
where
    S: ReadOnlySystem<In = Mapping<T>, Out = T>,
{
    fn name(&self) -> Cow<'static, str> {
        self.system.name()
    }

    fn initialize(&mut self, world: &mut World) {
        self.system.initialize(world);
    }

    fn map(&mut self, ctx: MappingCtx<T>) -> T {
        self.system.run_readonly(ctx.mapping, ctx.world)
    }
}

#[doc(hidden)]
pub struct MapperSystemMarker;

impl<T, M, S> IntoMapper<T, (MapperSystemMarker, M)> for S
where
    S: IntoSystem<Mapping<T>, T, M, System: ReadOnlySystem>,
{
    type Mapper = SystemMapper<S::System>;

    fn into_mapper(self) -> Self::Mapper {
        SystemMapper {
            system: IntoSystem::into_system(self),
        }
    }
}
