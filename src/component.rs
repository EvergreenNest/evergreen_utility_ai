//! Provides components for associating entities with flows, actions, and
//! storing their computed scores.

use std::sync::Arc;

use bevy_ecs::{
    batching::BatchingStrategy,
    component::{Component, ComponentId},
    entity::Entity,
    query::QueryState,
    world::{DeferredWorld, World},
};
use bevy_utils::{HashMap, HashSet, Parallel};
use parking_lot::Mutex;

use crate::{
    flow::WorldFlowExt,
    label::{
        ActionLabel, FlowLabel, InternedActionLabel, InternedFlowLabel, InternedScoreLabel,
        ScoreLabel,
    },
    score::Score,
    selector::{IntoSelector, Selector},
};

/// [`System`] that runs all entity associated flows in parallel.
///
/// [`System`]: bevy_ecs::system::System
pub fn run_all_entity_flows(world: &mut World, entities: &mut QueryState<(Entity, &EntityFlow)>) {
    type ComputedScoresQueue = Vec<(Entity, HashMap<InternedScoreLabel, Score>)>;

    let mut queue = Parallel::<ComputedScoresQueue>::default();

    let flows = entities
        .iter(world)
        .map(|(_, flow)| flow.0)
        .collect::<HashSet<_>>();

    // Make sure all of the entity associated flows are initialized so we can
    // skip initialization in the parallel loop.
    for &flow in &flows {
        world
            .try_flow_scope(flow, |world, flow| {
                flow.initialize(world);
            })
            .ok();
    }

    drop(flows);

    entities
        .par_iter(world)
        .batching_strategy(BatchingStrategy::new().min_batch_size(500))
        .for_each_init(
            || queue.borrow_local_mut(),
            |queue, (entity, label)| {
                let Some(flow) = world.get_flow(label.0) else {
                    return;
                };

                let scores = flow.run_readonly(world, entity);
                queue.push((entity, scores));
            },
        );

    for (entity, scores) in queue.drain() {
        world
            .commands()
            .entity(entity)
            .insert(ComputedScores(scores));
    }
}

/// A [`Component`] that associates an entity with a [`Flow`].
///
/// Use [`EntityCommandsFlowExt::run_flow`] to run this flow for an entity.
///
/// [`Flow`]: crate::flow::Flow
/// [`EntityCommandsFlowExt::run_flow`]: crate::command::EntityCommandsFlowExt::run_flow
#[derive(Component)]
#[require(ComputedScores)]
pub struct EntityFlow(pub InternedFlowLabel);

impl EntityFlow {
    /// Create a new [`EntityFlow`] with the given [`FlowLabel`].
    pub fn new(label: impl FlowLabel) -> Self {
        Self(label.intern())
    }
}

/// A [`Component`] that associates an entity with a [`Selector`].
#[derive(Component)]
#[component(on_insert = Self::on_insert)]
#[require(ComputedScores)]
pub struct ActionSelector(pub Arc<Mutex<dyn Selector>>);

impl ActionSelector {
    /// Create a new [`ActionSelector`] with the given [`Selector`].
    pub fn new<M>(selector: impl IntoSelector<M>) -> Self {
        Self(Arc::new(Mutex::new(selector.into_selector())))
    }

    fn on_insert(mut world: DeferredWorld, entity: Entity, _: ComponentId) {
        let selector = Arc::clone(&world.get::<ActionSelector>(entity).unwrap().0);
        world.commands().queue(move |world: &mut World| {
            selector.lock().initialize(world);
        });
    }
}

/// A [`Component`] that stores an entity's computed scores from their most
/// recent [`Flow`] evaluation.
///
/// [`Flow`]: crate::flow::Flow
#[derive(Component, Default)]
pub struct ComputedScores(HashMap<InternedScoreLabel, Score>);

impl ComputedScores {
    /// Get the [`Score`] associated with the given [`ScoreLabel`].
    pub fn get(&self, label: impl ScoreLabel) -> Option<Score> {
        self.0.get(&label.intern()).copied()
    }

    /// Insert a [`Score`] associated with the given [`ScoreLabel`].
    pub fn insert(&mut self, label: impl ScoreLabel, score: Score) -> Option<Score> {
        self.0.insert(label.intern(), score)
    }
}

/// A [`Component`] that associates an entity with a set of actions keyed by
/// labeled scores.
#[derive(Component)]
pub struct Actions {
    /// The actions to pick when the associated score is selected.
    actions: HashMap<InternedScoreLabel, InternedActionLabel>,
    /// The default action when no other actions are available.
    default: InternedActionLabel,
    /// The current action.
    current: InternedActionLabel,
}

impl Actions {
    /// Create a new [`Actions`] with the given default [`ActionLabel`].
    pub fn new(default: impl ActionLabel) -> Self {
        let default = default.intern();
        Self {
            actions: HashMap::new(),
            default,
            current: default,
        }
    }

    /// Adds an [`ActionLabel`] associated with the given [`ScoreLabel`].
    pub fn with(mut self, score: impl ScoreLabel, action: impl ActionLabel) -> Self {
        self.actions.insert(score.intern(), action.intern());
        self
    }

    /// Gets the [`ActionLabel`] associated with the given [`ScoreLabel`], if any.
    pub fn action(&self, score: impl ScoreLabel) -> Option<impl ActionLabel> {
        self.actions.get(&score.intern()).copied()
    }

    /// Gets the current [`ActionLabel`].
    pub fn current(&self) -> impl ActionLabel {
        self.current
    }

    /// Gets the default [`ActionLabel`].
    pub fn default(&self) -> impl ActionLabel {
        self.default
    }
}
