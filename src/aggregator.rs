//! Provides the [`Aggregator`] trait for aggregating scores from children
//! entities into a single score.

use alloc::{borrow::Cow, boxed::Box, format};
use core::marker::PhantomData;

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
mod median;
mod minimum;
mod product;
mod sum;
mod system;

pub use average::*;
pub use maximum::*;
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
pub trait IntoAggregator<Marker>: Sized {
    /// The type of [`Aggregator`] that this value will be converted into.
    type Aggregator: Aggregator;

    /// Converts this value into a [`Aggregator`].
    fn into_aggregator(self) -> Self::Aggregator;

    /// Maps the output score of this aggregator using the given [`Mapper`].
    fn map<M>(self, mapper: impl IntoMapper<Score, M>) -> impl Aggregator {
        struct MapAggregator<M, A> {
            mapper: M,
            aggregator: A,
        }

        impl<M, A> Aggregator for MapAggregator<M, A>
        where
            M: Mapper<Score>,
            A: Aggregator,
        {
            fn name(&self) -> Cow<'static, str> {
                Cow::Owned(format!(
                    "{}.map({})",
                    self.aggregator.name(),
                    self.mapper.name(),
                ))
            }

            fn initialize(&mut self, world: &mut World) {
                self.mapper.initialize(world);
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
    fn invert(self) -> impl Aggregator {
        struct InvertAggregator<A> {
            aggregator: A,
        }

        impl<A> Aggregator for InvertAggregator<A>
        where
            A: Aggregator,
        {
            fn name(&self) -> Cow<'static, str> {
                Cow::Owned(format!("{}.invert()", self.aggregator.name()))
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
    fn weight(self, weight: impl Into<Score>) -> impl Aggregator {
        struct WeightAggregator<A: Aggregator> {
            weight: Score,
            aggregator: A,
        }

        impl<A: Aggregator> Aggregator for WeightAggregator<A> {
            fn name(&self) -> Cow<'static, str> {
                Cow::Owned(format!(
                    "{}.weight({})",
                    self.aggregator.name(),
                    self.weight,
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
    fn curve(self, curve: impl Curve<Score> + Send + Sync + 'static) -> impl Aggregator {
        struct CurveAggregator<C: Curve<Score> + Send + Sync + 'static, A: Aggregator> {
            curve: C,
            aggregator: A,
        }

        impl<C: Curve<Score> + Send + Sync + 'static, A: Aggregator> Aggregator for CurveAggregator<C, A> {
            fn name(&self) -> Cow<'static, str> {
                Cow::Owned(format!(
                    "{}.curve({})",
                    self.aggregator.name(),
                    core::any::type_name::<C>(),
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
    fn curve_input(self, curve: impl Curve<Score> + Send + Sync + 'static) -> impl Aggregator {
        struct CurveInputAggregator<C: Curve<Score> + Send + Sync + 'static, A: Aggregator> {
            curve: C,
            aggregator: A,
        }

        impl<C: Curve<Score> + Send + Sync + 'static, A: Aggregator> Aggregator
            for CurveInputAggregator<C, A>
        {
            fn name(&self) -> Cow<'static, str> {
                Cow::Owned(format!(
                    "{}.curve_input({})",
                    self.aggregator.name(),
                    core::any::type_name::<C>(),
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
    fn threshold(self, threshold: impl Into<Score>) -> impl Aggregator {
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
    fn input_threshold(self, threshold: impl Into<Score>) -> impl Aggregator {
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
    fn score_children<C: Component + Scoreable>(self) -> impl Evaluator {
        struct ChildrenEvaluator<A: Aggregator, C: Component + Scoreable> {
            aggregator: A,
            _component: PhantomData<C>,
        }

        impl<A: Aggregator, C: Component + Scoreable> Evaluator for ChildrenEvaluator<A, C> {
            fn name(&self) -> Cow<'static, str> {
                Cow::Owned(format!(
                    "{}.score_children({})",
                    self.aggregator.name(),
                    core::any::type_name::<C>(),
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
    fn with_children<M>(self, other: impl IntoFlowNodeConfigs<M>) -> FlowNodeConfig {
        FlowNodeConfig::aggregator(self, other)
    }
}

/// All [`Aggregator`]s can be converted into themselves.
impl<A: Aggregator> IntoAggregator<()> for A {
    type Aggregator = A;

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
            average, geometric_mean, harmonic_mean, maximum, median, minimum, product, sum,
            Aggregation, AggregationCtx, Aggregator, IntoAggregator,
        },
        mapper::Mapping,
        score::Score,
    };

    #[test]
    fn average_aggregator() {
        let mut world = World::new();

        let mut aggregator = average();
        aggregator.initialize(&mut world);

        let output = aggregator.aggregate(AggregationCtx {
            world: &world,
            aggregation: Aggregation {
                target: Entity::PLACEHOLDER,
                scores: smallvec![0.1.into(), 0.2.into(), 0.3.into()],
            },
        });

        assert_eq!(output, Score::new(0.2));
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

        assert_eq!(output, Score::MAX);
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
    fn invert_aggregator() {
        let mut world = World::new();

        let mut aggregator = sum().invert();
        aggregator.initialize(&mut world);

        let output = aggregator.aggregate(AggregationCtx {
            world: &world,
            aggregation: Aggregation {
                target: Entity::PLACEHOLDER,
                scores: smallvec![0.1.into(), 0.1.into(), 0.1.into()],
            },
        });

        assert_eq!(output, Score::new(0.7));
    }

    #[test]
    fn map_aggregator() {
        let mut world = World::new();

        let mut aggregator = sum().map(|mapping: Mapping<Score>| mapping.value * 2.);
        aggregator.initialize(&mut world);

        let output = aggregator.aggregate(AggregationCtx {
            world: &world,
            aggregation: Aggregation {
                target: Entity::PLACEHOLDER,
                scores: smallvec![0.1.into(), 0.1.into(), 0.1.into()],
            },
        });

        assert_eq!(output, Score::new(0.6));
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

    #[test]
    fn geometric_mean_aggregator() {
        let mut world = World::new();

        let mut aggregator = geometric_mean();
        aggregator.initialize(&mut world);

        let output = aggregator.aggregate(AggregationCtx {
            world: &world,
            aggregation: Aggregation {
                target: Entity::PLACEHOLDER,
                scores: smallvec![0.1.into(), 0.2.into(), 0.3.into()],
            },
        });

        assert_eq!(output, Score::new(0.18171206));
    }

    #[test]
    fn harmonic_mean_aggregator() {
        let mut world = World::new();

        let mut aggregator = harmonic_mean();
        aggregator.initialize(&mut world);

        let output = aggregator.aggregate(AggregationCtx {
            world: &world,
            aggregation: Aggregation {
                target: Entity::PLACEHOLDER,
                scores: smallvec![0.1.into(), 0.2.into(), 0.3.into()],
            },
        });

        assert_eq!(output, Score::new(0.16363636));
    }

    #[test]
    fn median_aggregator() {
        let mut world = World::new();

        let mut aggregator = median();
        aggregator.initialize(&mut world);

        let output = aggregator.aggregate(AggregationCtx {
            world: &world,
            aggregation: Aggregation {
                target: Entity::PLACEHOLDER,
                scores: smallvec![0.3.into(), 0.15.into(), 0.5.into()],
            },
        });

        assert_eq!(output, Score::new(0.3));
    }

    #[test]
    fn minimum_aggregator() {
        let mut world = World::new();

        let mut aggregator = minimum();
        aggregator.initialize(&mut world);

        let output = aggregator.aggregate(AggregationCtx {
            world: &world,
            aggregation: Aggregation {
                target: Entity::PLACEHOLDER,
                scores: smallvec![0.3.into(), 0.15.into(), 0.5.into()],
            },
        });

        assert_eq!(output, Score::new(0.15));
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
    fn system_aggregator() {
        fn my_aggregator(agg: Aggregation) -> Score {
            agg.scores.into_iter().sum()
        }

        let mut world = World::new();

        let mut aggregator = my_aggregator.into_aggregator();
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
    fn threshold_aggregator() {
        let mut world = World::new();

        let mut aggregator = product().threshold(0.5);
        aggregator.initialize(&mut world);

        let output = aggregator.aggregate(AggregationCtx {
            world: &world,
            aggregation: Aggregation {
                target: Entity::PLACEHOLDER,
                scores: smallvec![0.6.into(), 0.7.into(), 0.8.into()],
            },
        });

        assert_eq!(output, Score::MIN);
    }

    #[test]
    fn input_threshold_aggregator() {
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

            assert_eq!(output, Score::MAX);
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

            assert_eq!(output, Score::MIN);
        }
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
}
