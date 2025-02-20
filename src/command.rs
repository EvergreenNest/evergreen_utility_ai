use bevy_ecs::{
    entity::Entity,
    query::QueryState,
    system::{EntityCommand, EntityCommands, Local},
    world::World,
};

use crate::{
    component::ScoreFlow,
    flow::WorldFlowExt,
    label::{FlowLabel, InternedFlowLabel},
};

/// [`System`] that runs the flows associated with entities.
///
/// [`System`]: bevy_ecs::system::System
pub fn run_entity_flows(
    world: &mut World,
    query: &mut QueryState<(Entity, &ScoreFlow)>,
    mut entity_flows: Local<Vec<(Entity, InternedFlowLabel)>>,
) {
    entity_flows.clear();
    entity_flows.extend(query.iter(world).map(|(entity, flow)| (entity, flow.0)));

    for &(entity, flow) in entity_flows.iter() {
        if let Err(e) = world.try_run_flow(flow, entity) {
            tracing::error!("Failed to run flow for entity {entity}: {e}");
        }
    }
}

/// Extension trait for [`EntityCommands`] that adds methods for running [`Flow`]s.
///
/// [`Flow`]: crate::flow::Flow
pub trait EntityCommandsFlowExt {
    /// Runs the flow with the given [`FlowLabel`] for the current entity.
    fn run_flow(&mut self, flow: impl FlowLabel) -> &mut Self;

    /// Runs the flow associated with the current entity.
    fn run_entity_flow(&mut self) -> &mut Self;
}

impl EntityCommandsFlowExt for EntityCommands<'_> {
    fn run_flow(&mut self, flow: impl FlowLabel) -> &mut Self {
        self.queue(RunFlow::new(flow))
    }

    fn run_entity_flow(&mut self) -> &mut Self {
        self.queue(RunEntityFlow)
    }
}

/// [`EntityCommand`] for running the flow with the given [`FlowLabel`] for the
/// current entity.
pub struct RunFlow(pub InternedFlowLabel);

impl RunFlow {
    /// Create a new [`RunFlow`] [`EntityCommand`] with the given [`FlowLabel`].
    pub fn new(flow: impl FlowLabel) -> Self {
        Self(flow.intern())
    }
}

impl EntityCommand for RunFlow {
    fn apply(self, entity: Entity, world: &mut World) {
        if let Err(e) = world.try_run_flow(self.0, entity) {
            tracing::error!("Failed to run flow for entity {entity}: {e}");
        }
    }
}

/// [`EntityCommand`] for running the flow associated with the current entity.
pub struct RunEntityFlow;

impl EntityCommand for RunEntityFlow {
    fn apply(self, entity: Entity, world: &mut World) {
        let Some(flow) = world.get::<ScoreFlow>(entity) else {
            tracing::error!("Entity {entity} does not have an associated flow");
            return;
        };
        if let Err(e) = world.try_run_flow(flow.0, entity) {
            tracing::error!("Failed to run flow for entity {entity}: {e}");
        }
    }
}
