//! Provides the [`Aggregator`] trait for aggregating scores from children
//! entities into a single score.

use std::{borrow::Cow, marker::PhantomData};

use bevy_ecs::{component::Component, entity::Entity, system::SystemInput, world::World};
use bevy_hierarchy::Children;
use bevy_math::Curve;
use smallvec::SmallVec;

use crate::{
    evaluator::{EvaluationCtx, Evaluator},
    flow::{FlowNodeConfig, IntoFlowNodeConfigs},
    mapper::{IntoMapper, Mapper, Mapping, MappingCtx},
    score::{Score, Scoreable},
};

mod average;
mod maximum;
mod mean;
mod median;
mod minimum;
mod product;
mod sum;
mod system;

pub use average::*;
pub use maximum::*;
pub use mean::*;
pub use median::*;
pub use minimum::*;
pub use product::*;
pub use sum::*;
pub use system::*;

/// Trait for types that view the target [`Entity`] in a [`World`] and children
/// [`Score`]s and aggregate them into a single [`Score`].
pub trait Aggregator: Send + Sync + 'static {
    /// Returns the name of the aggregator.
    fn name(&self) -> Cow<'static, str>;

    /// Initializes the aggregator using the given world.
    fn initialize(&mut self, world: &mut World) {
        let _ = world;
    }

    /// Aggregates the children scores of the target entity.
    fn aggregate(&mut self, ctx: AggregationCtx) -> Score;
}

/// Verifies that [`Aggregator`] is dyn-compatible.
const _: Option<Box<dyn Aggregator>> = None;

/// Trait for types that can be converted into an [`Aggregator`].
pub trait IntoAggregator<Marker> {
    /// The type of [`Aggregator`] that this value will be converted into.
    type Aggregator: Aggregator;

    /// Converts this value into a [`Aggregator`].
    fn into_aggregator(self) -> Self::Aggregator;

    /// Maps the output score of this aggregator using the given [`Mapper`].
    fn map<M>(self, mapper: impl IntoMapper<Score, M>) -> impl Aggregator
    where
        Self: Sized,
    {
        struct MapAggregator<M, C> {
            mapper: M,
            aggregator: C,
        }

        impl<M, C> Aggregator for MapAggregator<M, C>
        where
            M: Mapper<Score>,
            C: Aggregator,
        {
            fn name(&self) -> Cow<'static, str> {
                Cow::Owned(format!(
                    "map({}, {})",
                    self.mapper.name(),
                    self.aggregator.name()
                ))
            }

            fn initialize(&mut self, world: &mut World) {
                self.aggregator.initialize(world);
            }

            fn aggregate(&mut self, ctx: AggregationCtx) -> Score {
                let world = ctx.world;
                let target = ctx.aggregation.target;

                let value = self.aggregator.aggregate(ctx);
                self.mapper.map(MappingCtx {
                    world,
                    mapping: Mapping { target, value },
                })
            }
        }

        MapAggregator {
            mapper: mapper.into_mapper(),
            aggregator: self.into_aggregator(),
        }
    }

    /// Inverts the output score of this aggregator.
    fn invert(self) -> impl Aggregator
    where
        Self: Sized,
    {
        struct InvertAggregator<C> {
            aggregator: C,
        }

        impl<C> Aggregator for InvertAggregator<C>
        where
            C: Aggregator,
        {
            fn name(&self) -> Cow<'static, str> {
                Cow::Owned(format!("invert({})", self.aggregator.name()))
            }

            fn initialize(&mut self, world: &mut World) {
                self.aggregator.initialize(world);
            }

            fn aggregate(&mut self, ctx: AggregationCtx) -> Score {
                let score = self.aggregator.aggregate(ctx);
                Score::new(1. - score.get())
            }
        }

        InvertAggregator {
            aggregator: self.into_aggregator(),
        }
    }

    /// Multiplies the output score of this aggregator by the given weight.
    fn weight(self, weight: impl Into<Score>) -> impl Aggregator
    where
        Self: Sized,
    {
        struct WeightAggregator<C: Aggregator> {
            weight: Score,
            aggregator: C,
        }

        impl<C: Aggregator> Aggregator for WeightAggregator<C> {
            fn name(&self) -> Cow<'static, str> {
                Cow::Owned(format!(
                    "weight({}, {})",
                    self.weight,
                    self.aggregator.name(),
                ))
            }

            fn initialize(&mut self, world: &mut World) {
                self.aggregator.initialize(world);
            }

            fn aggregate(&mut self, ctx: AggregationCtx) -> Score {
                self.aggregator.aggregate(ctx) * self.weight
            }
        }

        WeightAggregator {
            weight: weight.into(),
            aggregator: self.into_aggregator(),
        }
    }

    /// Applies the given [`Curve`] to this aggregator's output score. If the
    /// curve cannot be sampled at the output score value, the aggregator
    /// returns [`Score::MIN`].
    fn curve(self, curve: impl Curve<Score> + Send + Sync + 'static) -> impl Aggregator
    where
        Self: Sized,
    {
        struct CurveAggregator<C: Curve<Score> + Send + Sync + 'static, Co: Aggregator> {
            curve: C,
            aggregator: Co,
        }

        impl<C: Curve<Score> + Send + Sync + 'static, Co: Aggregator> Aggregator
            for CurveAggregator<C, Co>
        {
            fn name(&self) -> Cow<'static, str> {
                Cow::Owned(format!(
                    "curve<{}>({})",
                    std::any::type_name::<C>(),
                    self.aggregator.name()
                ))
            }

            fn initialize(&mut self, world: &mut World) {
                self.aggregator.initialize(world);
            }

            fn aggregate(&mut self, ctx: AggregationCtx) -> Score {
                let score = self.aggregator.aggregate(ctx);
                self.curve.sample(score.get()).unwrap_or(Score::MIN)
            }
        }

        CurveAggregator {
            curve,
            aggregator: self.into_aggregator(),
        }
    }

    /// Applies the given [`Curve`] to this aggregator's input scores. If the
    /// curve cannot be sampled at any input score value, that score is set to
    /// [`Score::MIN`].
    fn curve_input(self, curve: impl Curve<Score> + Send + Sync + 'static) -> impl Aggregator
    where
        Self: Sized,
    {
        struct CurveInputAggregator<C: Curve<Score> + Send + Sync + 'static, Co: Aggregator> {
            curve: C,
            aggregator: Co,
        }

        impl<C: Curve<Score> + Send + Sync + 'static, Co: Aggregator> Aggregator
            for CurveInputAggregator<C, Co>
        {
            fn name(&self) -> Cow<'static, str> {
                Cow::Owned(format!(
                    "curve_input<{}>({})",
                    std::any::type_name::<C>(),
                    self.aggregator.name()
                ))
            }

            fn initialize(&mut self, world: &mut World) {
                self.aggregator.initialize(world);
            }

            fn aggregate(&mut self, ctx: AggregationCtx) -> Score {
                self.aggregator.aggregate(AggregationCtx {
                    world: ctx.world,
                    aggregation: Aggregation {
                        target: ctx.aggregation.target,
                        scores: ctx
                            .aggregation
                            .scores
                            .into_iter()
                            .flat_map(|score| self.curve.sample(score.get()))
                            .collect(),
                    },
                })
            }
        }

        CurveInputAggregator {
            curve,
            aggregator: self.into_aggregator(),
        }
    }

    /// Applies the given threshold to this aggregator's output score. If the
    /// output score is less than the threshold, the aggregator returns
    /// [`Score::MIN`].
    fn threshold(self, threshold: impl Into<Score>) -> impl Aggregator
    where
        Self: Sized,
    {
        struct OutputThresholdAggregator<A> {
            threshold: Score,
            aggregator: A,
        }

        impl<A: Aggregator> Aggregator for OutputThresholdAggregator<A> {
            fn name(&self) -> Cow<'static, str> {
                Cow::Owned(format!(
                    "{}.threshold({})",
                    self.aggregator.name(),
                    self.threshold,
                ))
            }

            fn initialize(&mut self, world: &mut World) {
                self.aggregator.initialize(world);
            }

            fn aggregate(&mut self, ctx: AggregationCtx) -> Score {
                let score = self.aggregator.aggregate(ctx);
                if score < self.threshold {
                    Score::MIN
                } else {
                    score
                }
            }
        }

        OutputThresholdAggregator {
            threshold: threshold.into(),
            aggregator: self.into_aggregator(),
        }
    }

    /// Applies the given threshold to this aggregator's input scores. If any
    /// input score is less than the threshold, the aggregator returns
    /// [`Score::MIN`].
    fn input_threshold(self, threshold: impl Into<Score>) -> impl Aggregator
    where
        Self: Sized,
    {
        struct InputThresholdAggregator<A> {
            threshold: Score,
            aggregator: A,
        }

        impl<A: Aggregator> Aggregator for InputThresholdAggregator<A> {
            fn name(&self) -> Cow<'static, str> {
                Cow::Owned(format!(
                    "{}.input_threshold({})",
                    self.aggregator.name(),
                    self.threshold,
                ))
            }

            fn initialize(&mut self, world: &mut World) {
                self.aggregator.initialize(world);
            }

            fn aggregate(&mut self, ctx: AggregationCtx) -> Score {
                if ctx
                    .aggregation
                    .scores
                    .iter()
                    .any(|&score| score < self.threshold)
                {
                    Score::MIN
                } else {
                    self.aggregator.aggregate(ctx)
                }
            }
        }

        InputThresholdAggregator {
            threshold: threshold.into(),
            aggregator: self.into_aggregator(),
        }
    }

    /// Converts this aggregator into a [`Evaluator`] that scores the given
    /// [`Component`] for the children entities of the target entity, and then
    /// aggregates the scores using this aggregator. If the target entity does
    /// not have any children with the component, the evaluator returns
    /// [`Score::MIN`].
    fn score_children<C: Component + Scoreable>(self) -> impl Evaluator
    where
        Self: Sized,
    {
        struct ChildrenEvaluator<Co: Aggregator, C: Component + Scoreable> {
            aggregator: Co,
            _component: PhantomData<C>,
        }

        impl<Co: Aggregator, C: Component + Scoreable> Evaluator for ChildrenEvaluator<Co, C> {
            fn name(&self) -> Cow<'static, str> {
                Cow::Owned(format!(
                    "children<{}>({})",
                    std::any::type_name::<C>(),
                    self.aggregator.name()
                ))
            }

            fn initialize(&mut self, world: &mut World) {
                self.aggregator.initialize(world);
            }

            fn evaluate(&mut self, ctx: EvaluationCtx) -> Score {
                // Get the children entities of the target entity.
                let Some(children) = ctx.world.get::<Children>(ctx.evaluation.target) else {
                    return Score::MIN;
                };
                // Score the children's components and collect them.
                let scores = children
                    .into_iter()
                    .flat_map(|&entity| ctx.world.get::<C>(entity).map(|c| c.score()))
                    .collect::<SmallVec<_>>();
                // Return early if none of the children have the component.
                if scores.is_empty() {
                    return Score::MIN;
                }
                // Evaluate the wrapped aggregator with the scores.
                self.aggregator.aggregate(AggregationCtx {
                    world: ctx.world,
                    aggregation: Aggregation {
                        target: ctx.evaluation.target,
                        scores,
                    },
                })
            }
        }

        ChildrenEvaluator {
            aggregator: self.into_aggregator(),
            _component: PhantomData::<C>,
        }
    }

    /// Sets the children of this aggregator to the given [`Aggregator`]s and/or [`Evaluator`]s.
    fn with_children<M>(self, other: impl IntoFlowNodeConfigs<M>) -> FlowNodeConfig
    where
        Self: Sized,
    {
        FlowNodeConfig::aggregator(self, other)
    }
}

/// All [`Aggregator`]s can be converted into themselves.
impl<C: Aggregator> IntoAggregator<()> for C {
    type Aggregator = C;

    fn into_aggregator(self) -> Self::Aggregator {
        self
    }
}

/// [`World`] and [`Aggregation`] pair passed to [`Aggregator`]s.
#[derive(Clone, Debug)]
pub struct AggregationCtx<'w> {
    /// The world in which the [`Aggregator`] is being evaluated.
    pub world: &'w World,
    /// The aggregation of scores being evaluated.
    pub aggregation: Aggregation,
}

/// [`SystemInput`] type for [`Aggregator`] systems.
#[derive(Clone, PartialEq, Debug)]
pub struct Aggregation {
    /// The entity that is being scored.
    pub target: Entity,
    /// The computed children scores.
    pub scores: SmallVec<[Score; 4]>,
}

impl SystemInput for Aggregation {
    type Param<'i> = Aggregation;
    type Inner<'i> = Aggregation;

    fn wrap(this: Self::Inner<'_>) -> Self::Param<'_> {
        this
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::{entity::Entity, world::World};
    use bevy_math::curve::FunctionCurve;
    use smallvec::smallvec;

    use crate::{
        aggregator::{
            maximum, product, sum, Aggregation, AggregationCtx, Aggregator, IntoAggregator,
        },
        score::Score,
    };

    #[test]
    fn all_or_nothing_aggregator() {
        let mut world = World::new();

        {
            let mut aggregator = sum().input_threshold(0.5);
            aggregator.initialize(&mut world);

            let output = aggregator.aggregate(AggregationCtx {
                world: &world,
                aggregation: Aggregation {
                    target: Entity::PLACEHOLDER,
                    scores: smallvec![0.6.into(), 0.7.into(), 0.8.into()],
                },
            });

            assert_eq!(output, Score::new(1.0));
        }

        {
            let mut aggregator = sum().input_threshold(0.9);
            aggregator.initialize(&mut world);

            let output = aggregator.aggregate(AggregationCtx {
                world: &world,
                aggregation: Aggregation {
                    target: Entity::PLACEHOLDER,
                    scores: smallvec![0.6.into(), 0.7.into(), 0.8.into()],
                },
            });

            assert_eq!(output, Score::new(0.0));
        }
    }

    #[test]
    fn curve_aggregator() {
        let mut world = World::new();

        let mut aggregator =
            sum().curve(FunctionCurve::new(Score::INTERVAL, |x| Score::new(x * 2.)));
        aggregator.initialize(&mut world);

        let output = aggregator.aggregate(AggregationCtx {
            world: &world,
            aggregation: Aggregation {
                target: Entity::PLACEHOLDER,
                scores: smallvec![0.2.into(), 0.3.into(), 0.2.into()],
            },
        });

        assert_eq!(output, Score::new(0.17));
    }

    #[test]
    fn curve_input_aggregator() {
        let mut world = World::new();

        let mut aggregator =
            sum().curve_input(FunctionCurve::new(Score::INTERVAL, |x| Score::new(x * 2.)));
        aggregator.initialize(&mut world);

        let output = aggregator.aggregate(AggregationCtx {
            world: &world,
            aggregation: Aggregation {
                target: Entity::PLACEHOLDER,
                scores: smallvec![0.15.into(), 0.1.into(), 0.12.into()],
            },
        });

        assert_eq!(output, Score::new(0.74));
    }

    #[test]
    fn product_aggregator() {
        let mut world = World::new();

        let mut aggregator = product();
        aggregator.initialize(&mut world);

        let output = aggregator.aggregate(AggregationCtx {
            world: &world,
            aggregation: Aggregation {
                target: Entity::PLACEHOLDER,
                scores: smallvec![0.3.into(), 0.15.into(), 0.5.into()],
            },
        });

        assert_eq!(output, Score::new(0.0225));
    }

    #[test]
    fn sum_aggregator() {
        let mut world = World::new();

        let mut aggregator = sum();
        aggregator.initialize(&mut world);

        let output = aggregator.aggregate(AggregationCtx {
            world: &world,
            aggregation: Aggregation {
                target: Entity::PLACEHOLDER,
                scores: smallvec![0.1.into(), 0.1.into(), 0.1.into()],
            },
        });

        assert_eq!(output, Score::new(0.3));
    }

    #[test]
    fn weight_aggregator() {
        let mut world = World::new();

        let mut aggregator = sum().weight(0.5);
        aggregator.initialize(&mut world);

        let output = aggregator.aggregate(AggregationCtx {
            world: &world,
            aggregation: Aggregation {
                target: Entity::PLACEHOLDER,
                scores: smallvec![0.1.into(), 0.1.into(), 0.1.into()],
            },
        });

        assert_eq!(output, Score::new(0.15));
    }

    #[test]
    fn maximum_aggregator() {
        let mut world = World::new();

        let mut aggregator = maximum();
        aggregator.initialize(&mut world);

        let output = aggregator.aggregate(AggregationCtx {
            world: &world,
            aggregation: Aggregation {
                target: Entity::PLACEHOLDER,
                scores: smallvec![0.3.into(), 0.6.into(), 0.8.into()],
            },
        });

        assert_eq!(output, Score::new(0.8));
    }
}
