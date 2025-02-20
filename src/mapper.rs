use std::borrow::Cow;

use bevy_ecs::{entity::Entity, system::SystemInput, world::World};

use crate::score::Score;

mod system;

pub use system::*;

/// Trait for types that view the target [`Entity`] in a [`World`] and maps a
/// value into a new value.
pub trait Mapper<T>: Send + Sync + 'static {
    /// Returns the name of the mapper.
    fn name(&self) -> Cow<'static, str>;

    /// Initializes the mapper using the given world.
    fn initialize(&mut self, world: &mut World) {
        let _ = world;
    }

    /// Maps the value using the given context.
    fn map(&mut self, ctx: MappingCtx<T>) -> T;
}

/// Verifies that [`Mapper`] is dyn-compatible.
const _: Option<Box<dyn Mapper<Score>>> = None;

/// Trait for types that can be converted into a [`Mapper`].
pub trait IntoMapper<T, Marker> {
    /// The type of [`Mapper`] that this value can be converted into.
    type Mapper: Mapper<T>;

    /// Converts this value into a [`Mapper`].
    fn into_mapper(self) -> Self::Mapper;
}

/// All [`Mapper`]s can be converted into themselves.
impl<T, M: Mapper<T>> IntoMapper<T, ()> for M {
    type Mapper = M;

    fn into_mapper(self) -> Self::Mapper {
        self
    }
}

/// [`World`] and [`Mapping`] pair passed to [`Mapper`]s.
#[derive(Clone, Debug)]
pub struct MappingCtx<'a, T> {
    /// The world in which the [`Mapper`] is being mapped.
    pub world: &'a World,
    /// The mapping being performed.
    pub mapping: Mapping<T>,
}

/// [`SystemInput`] type for [`Mapper`] systems.
#[derive(Clone, PartialEq, Debug)]
pub struct Mapping<T> {
    /// The entity being mapped.
    pub target: Entity,
    /// The value being mapped.
    pub value: T,
}

impl<T> SystemInput for Mapping<T> {
    type Param<'i> = Mapping<T>;
    type Inner<'i> = Mapping<T>;

    fn wrap(this: Self::Inner<'_>) -> Self::Param<'_> {
        this
    }
}
