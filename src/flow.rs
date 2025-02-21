//! Provides the [`Flow`] type for defining a collection of [`Aggregator`] and
//! [`Evaluator`] nodes and running them in topological order.

use std::hash::Hash;

use bevy_ecs::{entity::Entity, system::Resource, world::World};
use bevy_utils::HashMap;
use parking_lot::Mutex;
use petgraph::{algo::toposort, prelude::DiGraphMap};
use smallvec::SmallVec;
use thiserror::Error;
use tracing::warn;

use crate::{
    aggregator::{Aggregation, AggregationCtx, Aggregator},
    evaluator::{Evaluation, EvaluationCtx, Evaluator},
    label::{FlowLabel, InternedFlowLabel, InternedScoreLabel, ScoreLabel},
    score::Score,
};

mod config;

pub use config::*;

/// [`Resource`] that stores [`Flow`]s mapped to [`FlowLabel`]s, excluding the
/// current running [`Flow`].
#[derive(Resource, Default)]
pub struct Flows {
    inner: HashMap<InternedFlowLabel, Flow>,
}

impl Flows {
    /// Returns a reference to the [`Flow`] with the given label, creating it if
    /// it doesn't exist.
    pub fn entry(&mut self, label: impl FlowLabel) -> &mut Flow {
        self.inner
            .entry(label.intern())
            .or_insert_with(|| Flow::new(label))
    }

    /// Inserts a [`Flow`].
    pub fn insert(&mut self, flow: Flow) -> Option<Flow> {
        self.inner.insert(flow.label, flow)
    }

    /// Removes a [`Flow`].
    pub fn remove(&mut self, label: impl FlowLabel) -> Option<Flow> {
        self.inner.remove(&label.intern())
    }

    /// Adds one or more nodes to the [`Flow`] matching the given [`FlowLabel`].
    pub fn add_nodes<M>(
        &mut self,
        label: impl FlowLabel,
        nodes: impl IntoFlowNodeConfigs<M>,
    ) -> &mut Self {
        self.entry(label).add_nodes(nodes);
        self
    }
}

/// A collection of [`Aggregator`] and [`Evaluator`] nodes, and the metadata
/// needed to run them in topological order.
pub struct Flow {
    label: InternedFlowLabel,
    graph: FlowGraph,
}

impl Flow {
    /// Constructs an empty flow.
    pub fn new(label: impl FlowLabel) -> Self {
        Self {
            label: label.intern(),
            graph: FlowGraph::default(),
        }
    }

    /// Add a collection of nodes to the flow.
    pub fn add_nodes<M>(&mut self, nodes: impl IntoFlowNodeConfigs<M>) -> &mut Self {
        self.add_nodes_with_parent(None, nodes);
        self
    }

    fn add_nodes_with_parent<M>(
        &mut self,
        parent: Option<NodeId>,
        nodes: impl IntoFlowNodeConfigs<M>,
    ) {
        let configs = nodes.into_configs().0;
        for config in configs {
            if parent.is_none() && config.label.is_none() {
                // We skip inserting the node into the graph if it has no parent
                // and no label. Having neither means the output score of the
                // node would not be used.
                tracing::warn!(
                    "{} {} has no label or parent node, so it wasn't added to the {:?} flow.",
                    config.node.kind(),
                    config.node.name(),
                    self.label
                );
                continue;
            }

            let (node, children) = self.graph.add_node(parent, config.node);

            if let Some(label) = config.label {
                self.add_label(label, node);
            }

            if let Some(children) = children {
                for config in children.0 {
                    self.add_nodes_with_parent(Some(node), config);
                }
            }
        }
    }

    fn add_label(&mut self, label: impl ScoreLabel, node: NodeId) {
        let label = label.intern();
        if let Some(&nid) = self.graph.labels.get(&label) {
            let (kind, name) = match nid {
                NodeId::Evaluator(e) => ("evaluator", self.graph.evaluators[e].lock().name()),
                NodeId::Aggregator(a) => ("aggregator", self.graph.aggregators[a].lock().name()),
            };
            tracing::error!(
                "Label {label:?} is already associated with {kind} {name} in the {:?} flow. It was not overwritten.",
                self.label
            );
        } else {
            self.graph.labeled.insert(node, label);
            self.graph.labels.insert(label, node);
        }
    }

    /// Initializes the flow if necessary and runs it, returning the scores of
    /// all labeled nodes.
    pub fn run(&mut self, world: &mut World, target: Entity) -> HashMap<InternedScoreLabel, Score> {
        self.initialize(world);
        self.run_readonly(world, target)
    }

    /// Runs the flow, returning the scores of all labeled nodes.
    ///
    /// # Panics
    ///
    /// If the flow was not initialized before running.
    pub fn run_readonly(
        &self,
        world: &World,
        target: Entity,
    ) -> HashMap<InternedScoreLabel, Score> {
        assert!(
            self.graph.uninitialized.is_empty(),
            "flow {:?} was not initialized before running",
            self.label
        );

        let mut labeled_scores = HashMap::with_capacity(self.graph.labels.len());
        // Holds the intermediate child scores for each aggregator node.
        let mut aggregator_child_scores = HashMap::<NodeId, SmallVec<[Score; 4]>>::with_capacity(
            self.graph.dependency.node_count(),
        );

        for &node in &self.graph.dependency_toposort {
            let score = match node {
                NodeId::Evaluator(eval_idx) => {
                    let mut evaluator = self.graph.evaluators[eval_idx].lock();

                    evaluator.evaluate(EvaluationCtx {
                        world,
                        evaluation: Evaluation { target },
                    })
                }
                NodeId::Aggregator(aggr_idx) => {
                    let scores = aggregator_child_scores
                        .remove(&node)
                        .expect("aggregator node was not scored before its children");
                    let mut aggregator = self.graph.aggregators[aggr_idx].lock();

                    aggregator.aggregate(AggregationCtx {
                        world,
                        aggregation: Aggregation { target, scores },
                    })
                }
            };

            if let Some(label) = self.graph.labeled.get(&node) {
                labeled_scores.insert(*label, score);
            }

            let parent = self.graph.dependency.neighbors(node).next();

            if let Some(parent) = parent {
                aggregator_child_scores
                    .entry(parent)
                    .or_default()
                    .push(score);
            }
        }

        labeled_scores
    }

    /// Initializes all evaluators and aggregators in the flow.
    pub fn initialize(&mut self, world: &mut World) {
        self.graph.initialize(world);
    }
}

/// Stores all nodes in a flow graph and their dependency metadata.
#[derive(Default)]
pub struct FlowGraph {
    /// All evaluator nodes in the [`Flow`]. Any [`NodeId::Evaluator`] value
    /// must be an index into this [`Vec`].
    evaluators: Vec<Mutex<Box<dyn Evaluator>>>,
    /// All aggregator nodes in the [`Flow`]. Any [`NodeId::Aggregator`] value
    /// must be an index into this [`Vec`].
    aggregators: Vec<Mutex<Box<dyn Aggregator>>>,
    /// [`Evaluator`]s/[`Aggregator`]s that have not been initialized yet.
    uninitialized: Vec<NodeId>,
    /// All labeled nodes in the [`Flow`].
    labeled: HashMap<NodeId, InternedScoreLabel>,
    /// All labels in the [`Flow`]. This is a reverse mapping of [`FlowGraph::labeled`].
    labels: HashMap<InternedScoreLabel, NodeId>,
    /// Directed acyclic graph of node dependencies (which nodes have to run before which other nodes).
    dependency: DiGraphMap<NodeId, ()>,
    /// Topological sort of the dependency graph.
    dependency_toposort: Vec<NodeId>,
}

impl FlowGraph {
    /// Initializes all evaluators and aggregators in the flow.
    pub fn initialize(&mut self, world: &mut World) {
        for id in self.uninitialized.drain(..) {
            match id {
                NodeId::Evaluator(i) => self.evaluators[i].lock().initialize(world),
                NodeId::Aggregator(i) => self.aggregators[i].lock().initialize(world),
            }
        }
    }

    /// Adds an individual node to the [`FlowGraph`] and returns its [`NodeId`]
    /// and children, if any.
    fn add_node(
        &mut self,
        parent: Option<NodeId>,
        node: FlowNode,
    ) -> (NodeId, Option<FlowNodeConfigs>) {
        let (node, children) = match node {
            FlowNode::Aggregator {
                aggregator,
                children,
            } => {
                let node = NodeId::Aggregator(self.aggregators.len());
                self.aggregators.push(Mutex::new(aggregator));
                self.uninitialized.push(node);
                (node, Some(children))
            }
            FlowNode::Evaluator { evaluator } => {
                let node = NodeId::Evaluator(self.evaluators.len());
                self.evaluators.push(Mutex::new(evaluator));
                self.uninitialized.push(node);
                (node, None)
            }
        };

        if let Some(parent) = parent {
            self.dependency.add_edge(node, parent, ());
        } else {
            self.dependency.add_node(node);
        }

        self.dependency_toposort =
            toposort(&self.dependency, None).unwrap_or_else(|_| unreachable!());

        (node, children)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
enum NodeId {
    /// Index into [`FlowGraph::evaluators`].
    Evaluator(usize),
    /// Index into [`FlowGraph::aggregators`].
    Aggregator(usize),
}

/// [`World`] extension trait for working with [`Flow`]s.
pub trait WorldFlowExt {
    /// Adds a collection of nodes to the flow with the given label.
    ///
    /// If the flow does not exist, it will be created.
    fn add_nodes<M>(
        &mut self,
        label: impl FlowLabel,
        nodes: impl IntoFlowNodeConfigs<M>,
    ) -> &mut Self;

    /// Tries to run the flow with the given label, returning the scores of all
    /// labeled nodes.
    ///
    /// # Errors
    ///
    /// Returns [`TryRunFlowError`] if the flow with the given label does not
    /// exist.
    fn try_run_flow(
        &mut self,
        label: impl FlowLabel,
        target: Entity,
    ) -> Result<HashMap<InternedScoreLabel, Score>, TryRunFlowError> {
        self.try_flow_scope(label, |world, flow| flow.run(world, target))
    }

    /// Runs the flow with the given label, returning the scores of all labeled
    /// nodes.
    ///
    /// # Panics
    ///
    /// If the flow does not exist.
    #[must_use]
    fn run_flow(
        &mut self,
        label: impl FlowLabel,
        target: Entity,
    ) -> HashMap<InternedScoreLabel, Score> {
        self.flow_scope(label, |world, flow| flow.run(world, target))
    }

    /// Pulls the flow with the given label out of the [`Flows`] resource,
    /// provides it to the closure, and then re-inserts it into the resource.
    ///
    /// # Errors
    ///
    /// Returns [`TryRunFlowError`] if the flow with the given label does not
    /// exist.
    fn try_flow_scope<R>(
        &mut self,
        label: impl FlowLabel,
        f: impl FnOnce(&mut World, &mut Flow) -> R,
    ) -> Result<R, TryRunFlowError>;

    /// Pulls the flow with the given label out of the [`Flows`] resource,
    /// provides it to the closure, and then re-inserts it into the resource.
    ///
    /// # Panics
    ///
    /// If the flow with the given label does not exist.
    fn flow_scope<R>(
        &mut self,
        label: impl FlowLabel,
        f: impl FnOnce(&mut World, &mut Flow) -> R,
    ) -> R {
        self.try_flow_scope(label, f)
            .unwrap_or_else(|e| panic!("{e}"))
    }

    /// Returns a reference to the flow with the given label.
    fn get_flow(&self, label: impl FlowLabel) -> Option<&Flow>;
}

impl WorldFlowExt for World {
    fn add_nodes<M>(
        &mut self,
        label: impl FlowLabel,
        nodes: impl IntoFlowNodeConfigs<M>,
    ) -> &mut Self {
        let mut flows = self.get_resource_or_init::<Flows>();
        flows.add_nodes(label, nodes);
        self
    }

    fn try_flow_scope<R>(
        &mut self,
        label: impl FlowLabel,
        f: impl FnOnce(&mut World, &mut Flow) -> R,
    ) -> Result<R, TryRunFlowError> {
        let label = label.intern();
        let Some(mut flow) = self
            .get_resource_mut::<Flows>()
            .and_then(|mut flows| flows.remove(label))
        else {
            return Err(TryRunFlowError(label));
        };

        let value = f(self, &mut flow);
        let old = self.resource_mut::<Flows>().insert(flow);
        if old.is_some() {
            warn!("Flow `{label:?} was inserted during a call to `World::try_flow_scope`: its value has been overwritten");
        }

        Ok(value)
    }

    fn get_flow(&self, label: impl FlowLabel) -> Option<&Flow> {
        self.get_resource::<Flows>()
            .and_then(|flows| flows.inner.get(&label.intern()))
    }
}

/// Error type returned when trying to run a flow that does not exist.
#[derive(Error, Debug)]
#[error("The flow with the label {0:?} was not found.")]
pub struct TryRunFlowError(pub InternedFlowLabel);
