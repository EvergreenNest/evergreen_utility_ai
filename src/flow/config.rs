use alloc::{borrow::Cow, boxed::Box, vec, vec::Vec};

use variadics_please::all_tuples_with_size;

use crate::{
    aggregator::{AggregationCtx, Aggregator, IntoAggregator},
    evaluator::{Evaluator, IntoEvaluator},
    label::{InternedScoreLabel, ScoreLabel},
    score::Score,
};

/// Configuration for a flow node.
pub struct FlowNodeConfig {
    /// The node to register.
    pub(super) node: FlowNode,
    /// The [`ScoreLabel`] to associate with this node, if any.
    pub(super) label: Option<InternedScoreLabel>,
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

pub(super) enum FlowNode {
    /// An aggregator node and its children aggregator and/or evaluator nodes.
    Aggregator {
        /// The [`Aggregator`] to register.
        aggregator: Box<dyn Aggregator>,
        /// The [`Aggregator`]s and/or [`Evaluator`]s to register as a child to
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
        struct DifferenceAggregator;

        impl Aggregator for DifferenceAggregator {
            fn name(&self) -> Cow<'static, str> {
                Cow::Borrowed("difference")
            }

            fn aggregate(&mut self, ctx: AggregationCtx) -> Score {
                let [a, b] = [ctx.aggregation.scores[0], ctx.aggregation.scores[1]];
                a - b
            }
        }

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

impl<Marker, E> IntoFlowNodeConfig<(EvaluatorConfigMarker, Marker)> for E
where
    E: IntoEvaluator<Marker>,
{
    fn into_config(self) -> FlowNodeConfig {
        FlowNodeConfig::evaluator(self)
    }
}

/// A collection of [`FlowNodeConfig`]s.
pub struct FlowNodeConfigs(pub(super) Vec<FlowNodeConfig>);

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

impl<Marker, E> IntoFlowNodeConfigs<(EvaluatorConfigMarker, Marker)> for E
where
    E: IntoEvaluator<Marker>,
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
