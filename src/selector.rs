//! Provides the [`Selector`] trait for selecting actions based on computed scores.

use alloc::{borrow::Cow, boxed::Box};

use bevy_ecs::{entity::Entity, system::SystemInput, world::World};

use crate::{
    component::{Actions, ComputedScores},
    label::InternedActionLabel,
};

mod system;

pub use system::*;

/// Trait for types that select an action based on computed scores.
pub trait Selector: Send + Sync + 'static {
    /// Returns the name of the selector.
    fn name(&self) -> Cow<'static, str>;

    /// Initializes the selector using the given world.
    fn initialize(&mut self, world: &mut World) {
        let _ = world;
    }

    /// Selects an action label for the given selection context.
    fn select(&mut self, ctx: SelectionCtx) -> Option<InternedActionLabel>;
}

/// Verifies that [`Selector`] is dyn-compatible.
const _: Option<Box<dyn Selector>> = None;

/// Trait for types that can be converted into a [`Selector`].
pub trait IntoSelector<Marker> {
    /// The type of [`Selector`] that this value will be converted into.
    type Selector: Selector;

    /// Converts this value into a [`Selector`].
    fn into_selector(self) -> Self::Selector;
}

/// All [`Selector`]s can be converted into themselves.
impl<S: Selector> IntoSelector<()> for S {
    type Selector = S;

    fn into_selector(self) -> Self::Selector {
        self
    }
}

/// The context passed to [`Selector`]s when selecting an action.
pub struct SelectionCtx<'w, 's> {
    /// The world state.
    pub world: &'w World,
    /// The data associated with the selection.
    pub selection: Selection<'s>,
}

/// [`SystemInput`] type for [`Selector`] systems.
pub struct Selection<'s> {
    /// The entity that is being selected for.
    pub target: Entity,
    /// The computed scores for the target entity.
    pub scores: &'s ComputedScores,
    /// The actions that can be selected.
    pub actions: &'s Actions,
}

impl SystemInput for Selection<'_> {
    type Param<'i> = Selection<'i>;
    type Inner<'i> = Selection<'i>;

    fn wrap(this: Self::Inner<'_>) -> Self::Param<'_> {
        this
    }
}
