//! Provides the [`Flow`] type for defining a collection of [`Aggregator`] and
//! [`Evaluator`] nodes and running them in topological order.

use std::{borrow::Cow, collections::HashMap, hash::Hash};

use bevy_ecs::{entity::Entity, system::Resource, world::World};
use petgraph::{algo::toposort, prelude::DiGraphMap, Direction};
use smallvec::SmallVec;
use thiserror::Error;
use tracing::warn;
use variadics_please::all_tuples_with_size;

use crate::{
    aggregator::{Aggregation, AggregationCtx, Aggregator, DifferenceAggregator, IntoAggregator},
    evaluator::{Evaluation, EvaluationCtx, Evaluator, IntoEvaluator},
    label::{FlowLabel, InternedFlowLabel, InternedScoreLabel, ScoreLabel},
    score::Score,
};

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
                NodeId::Evaluator(e) => ("evaluator", self.graph.evaluators[e].name()),
                NodeId::Aggregator(a) => ("aggregator", self.graph.aggregators[a].name()),
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
        &mut self,
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
                    let evaluator = &mut self.graph.evaluators[eval_idx];

                    evaluator.evaluate(EvaluationCtx {
                        world,
                        evaluation: Evaluation { target },
                    })
                }
                NodeId::Aggregator(aggr_idx) => {
                    let aggregator = &mut self.graph.aggregators[aggr_idx];

                    let scores = aggregator_child_scores
                        .remove(&node)
                        .expect("aggregator node was not scored before its children");

                    aggregator.aggregate(AggregationCtx {
                        world,
                        aggregation: Aggregation { target, scores },
                    })
                }
            };

            if let Some(label) = self.graph.labeled.get(&node) {
                labeled_scores.insert(*label, score);
            }

            let parent = self
                .graph
                .dependency
                .neighbors_directed(node, Direction::Outgoing)
                .next();

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
    evaluators: Vec<Box<dyn Evaluator>>,
    /// All aggregator nodes in the [`Flow`]. Any [`NodeId::Aggregator`] value
    /// must be an index into this [`Vec`].
    aggregators: Vec<Box<dyn Aggregator>>,
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
                NodeId::Evaluator(i) => self.evaluators[i].initialize(world),
                NodeId::Aggregator(i) => self.aggregators[i].initialize(world),
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
                self.aggregators.push(aggregator);
                self.uninitialized.push(node);
                (node, Some(children))
            }
            FlowNode::Evaluator { evaluator } => {
                let node = NodeId::Evaluator(self.evaluators.len());
                self.evaluators.push(evaluator);
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

/// Configuration for a flow node.
pub struct FlowNodeConfig {
    /// The node to register.
    node: FlowNode,
    /// The [`ScoreLabel`] to associate with this node, if any.
    label: Option<InternedScoreLabel>,
}

impl FlowNodeConfig {
    /// Constructs a new config with the given [`Aggregator`] and children.
    pub fn aggregator<MC, MN>(
        aggregator: impl IntoAggregator<MC>,
        children: impl IntoFlowNodeConfigs<MN>,
    ) -> Self {
        Self {
            node: FlowNode::Aggregator {
                aggregator: Box::new(aggregator.into_aggregator()),
                children: children.into_configs(),
            },
            label: None,
        }
    }

    /// Constructs a new config with the given [`Evaluator`].
    pub fn evaluator<M>(evaluator: impl IntoEvaluator<M>) -> Self {
        Self {
            node: FlowNode::Evaluator {
                evaluator: Box::new(evaluator.into_evaluator()),
            },
            label: None,
        }
    }

    /// Labels this aggregator or evaluator with the given [`ScoreLabel`].
    pub fn label(mut self, label: impl ScoreLabel) -> Self {
        self.label = Some(label.intern());
        self
    }
}

enum FlowNode {
    /// An aggregator node and its children aggregator and/or evaluator nodes.
    Aggregator {
        /// The [`Aggregator`] to register.
        aggregator: Box<dyn Aggregator>,
        /// The children [`Aggregator`]s and/or [`Evaluator`]s to register for
        /// this aggregator.
        children: FlowNodeConfigs,
    },
    /// An evaluator node.
    Evaluator {
        /// The [`Evaluator`] to register.
        evaluator: Box<dyn Evaluator>,
    },
}

impl FlowNode {
    /// Returns the kind of the [`FlowNode`].
    pub fn kind(&self) -> &'static str {
        match self {
            FlowNode::Aggregator { .. } => "Aggregator",
            FlowNode::Evaluator { .. } => "Evaluator",
        }
    }

    /// Returns the name of the [`FlowNode`].
    pub fn name(&self) -> Cow<'static, str> {
        match self {
            FlowNode::Aggregator { aggregator, .. } => aggregator.name(),
            FlowNode::Evaluator { evaluator } => evaluator.name(),
        }
    }
}

/// Trait for types that can be converted into a [`FlowNodeConfig`].
pub trait IntoFlowNodeConfig<Marker> {
    /// Converts this value into a [`FlowNodeConfig`].
    fn into_config(self) -> FlowNodeConfig;

    /// Returns a [`FlowNodeConfig`] for an [`Aggregator`] that computes the
    /// difference of this value and the other.
    fn difference<M>(self, other: impl IntoFlowNodeConfig<M>) -> FlowNodeConfig
    where
        Self: Sized,
    {
        FlowNodeConfig::aggregator(
            DifferenceAggregator,
            (self.into_config(), other.into_config()),
        )
    }
}

impl IntoFlowNodeConfig<()> for FlowNodeConfig {
    fn into_config(self) -> FlowNodeConfig {
        self
    }
}

#[doc(hidden)]
pub struct EvaluatorConfigMarker;

impl<Marker, P> IntoFlowNodeConfig<(EvaluatorConfigMarker, Marker)> for P
where
    P: IntoEvaluator<Marker>,
{
    fn into_config(self) -> FlowNodeConfig {
        FlowNodeConfig::evaluator(self)
    }
}

/// A collection of [`FlowNodeConfig`]s.
pub struct FlowNodeConfigs(Vec<FlowNodeConfig>);

/// Trait for types that can be converted into a [`FlowNodeConfigs`].
pub trait IntoFlowNodeConfigs<Marker> {
    /// Converts this value into a [`FlowNodeConfigs`].
    fn into_configs(self) -> FlowNodeConfigs;
}

impl IntoFlowNodeConfigs<()> for FlowNodeConfig {
    fn into_configs(self) -> FlowNodeConfigs {
        FlowNodeConfigs(vec![self])
    }
}

impl IntoFlowNodeConfigs<()> for FlowNodeConfigs {
    fn into_configs(self) -> FlowNodeConfigs {
        self
    }
}

impl<Marker, P> IntoFlowNodeConfigs<(EvaluatorConfigMarker, Marker)> for P
where
    P: IntoEvaluator<Marker>,
{
    fn into_configs(self) -> FlowNodeConfigs {
        FlowNodeConfigs(vec![FlowNodeConfig::evaluator(self)])
    }
}

#[doc(hidden)]
pub struct FlowNodeConfigTupleMarker;

macro_rules! impl_score_system_collection {
    ($N:expr, $(#[$meta:meta])* $(($param: ident, $sys: ident)),*) => {
        $(#[$meta])*
        impl<$($param, $sys),*> IntoFlowNodeConfigs<(FlowNodeConfigTupleMarker, $($param,)*)> for ($($sys,)*)
        where
            $($sys: IntoFlowNodeConfigs<$param>),*
        {
            fn into_configs(self) -> FlowNodeConfigs {
                #[allow(non_snake_case, unused_variables)]
                let ($($sys,)*) = self;
                let mut configs = Vec::with_capacity($N);
                $(configs.extend($sys.into_configs().0);)*
                FlowNodeConfigs(configs)
            }
        }
    };
}

all_tuples_with_size!(impl_score_system_collection, 1, 20, P, S);

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
}

/// Error type returned when trying to run a flow that does not exist.
#[derive(Error, Debug)]
#[error("The flow with the label {0:?} was not found.")]
pub struct TryRunFlowError(pub InternedFlowLabel);
