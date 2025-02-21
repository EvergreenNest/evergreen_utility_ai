//! Provides the [`Evaluator`] trait for evaluating target [`Entity`]s in a
//! [`World`].

use alloc::{borrow::Cow, boxed::Box, format};

use bevy_ecs::{entity::Entity, system::SystemInput, world::World};
use bevy_math::Curve;

use crate::{
    flow::FlowNodeConfig,
    label::ScoreLabel,
    mapper::{IntoMapper, Mapper, Mapping, MappingCtx},
    score::Score,
};

mod constant;
mod parent;
mod resource;
mod system;
mod target;

pub use constant::*;
pub use parent::*;
pub use resource::*;
pub use system::*;
pub use target::*;

/// Trait for types that view the target [`Entity`] in a [`World`] and return a
/// [`Score`].
pub trait Evaluator: Send + Sync + 'static {
    /// Returns the name of the evaluator.
    fn name(&self) -> Cow<'static, str>;

    /// Initializes the evaluator using the given world.
    fn initialize(&mut self, world: &mut World) {
        let _ = world;
    }

    /// Evaluates the evaluator with the given context.
    fn evaluate(&mut self, ctx: EvaluationCtx) -> Score;
}

/// Verifies that [`Evaluator`] is dyn-compatible.
const _: Option<Box<dyn Evaluator>> = None;

/// Trait for types that can be converted into a [`Evaluator`].
pub trait IntoEvaluator<Marker>: Sized {
    /// The type of [`Evaluator`] that this value will be converted into.
    type Evaluator: Evaluator;

    /// Converts this value into a [`Evaluator`].
    fn into_evaluator(self) -> Self::Evaluator;

    /// Maps the output score of this evaluator using the given [`Mapper`].
    fn map<M>(self, mapper: impl IntoMapper<Score, M>) -> impl Evaluator {
        struct MapEvaluator<M, E> {
            mapper: M,
            evaluator: E,
        }

        impl<M, E> Evaluator for MapEvaluator<M, E>
        where
            M: Mapper<Score>,
            E: Evaluator,
        {
            fn name(&self) -> Cow<'static, str> {
                Cow::Owned(format!(
                    "{}.map({})",
                    self.evaluator.name(),
                    self.mapper.name(),
                ))
            }

            fn initialize(&mut self, world: &mut World) {
                self.evaluator.initialize(world);
            }

            fn evaluate(&mut self, ctx: EvaluationCtx) -> Score {
                let world = ctx.world;
                let target = ctx.evaluation.target;

                let value = self.evaluator.evaluate(ctx);
                self.mapper.map(MappingCtx {
                    world,
                    mapping: Mapping { target, value },
                })
            }
        }

        MapEvaluator {
            mapper: mapper.into_mapper(),
            evaluator: self.into_evaluator(),
        }
    }

    /// Inverts the output score of this evaluator.
    fn invert(self) -> impl Evaluator {
        struct InvertEvaluator<E> {
            evaluator: E,
        }

        impl<E> Evaluator for InvertEvaluator<E>
        where
            E: Evaluator,
        {
            fn name(&self) -> Cow<'static, str> {
                Cow::Owned(format!("{}.invert()", self.evaluator.name()))
            }

            fn initialize(&mut self, world: &mut World) {
                self.evaluator.initialize(world);
            }

            fn evaluate(&mut self, ctx: EvaluationCtx) -> Score {
                let score = self.evaluator.evaluate(ctx);
                Score::new(1. - score.get())
            }
        }

        InvertEvaluator {
            evaluator: self.into_evaluator(),
        }
    }

    /// Multiplies the output score of this evaluator by the given weight.
    fn weight(self, weight: impl Into<Score>) -> impl Evaluator {
        struct WeightEvaluator<E: Evaluator> {
            weight: Score,
            evaluator: E,
        }

        impl<E: Evaluator> Evaluator for WeightEvaluator<E> {
            fn name(&self) -> Cow<'static, str> {
                Cow::Owned(format!("{}.weight({})", self.evaluator.name(), self.weight))
            }

            fn initialize(&mut self, world: &mut World) {
                self.evaluator.initialize(world);
            }

            fn evaluate(&mut self, ctx: EvaluationCtx) -> Score {
                self.evaluator.evaluate(ctx) * self.weight
            }
        }

        WeightEvaluator {
            weight: weight.into(),
            evaluator: self.into_evaluator(),
        }
    }

    /// Applies the given [`Curve`] to this evaluator's output score. If the
    /// curve cannot be sampled at the output score value, the evaluator returns
    /// [`Score::MIN`].
    fn curve(self, curve: impl Curve<Score> + Send + Sync + 'static) -> impl Evaluator {
        struct CurveEvaluator<C: Curve<Score> + Send + Sync + 'static, E: Evaluator> {
            curve: C,
            evaluator: E,
        }

        impl<C: Curve<Score> + Send + Sync + 'static, E: Evaluator> Evaluator for CurveEvaluator<C, E> {
            fn name(&self) -> Cow<'static, str> {
                Cow::Owned(format!(
                    "{}.curve({})",
                    self.evaluator.name(),
                    core::any::type_name::<C>(),
                ))
            }

            fn initialize(&mut self, world: &mut World) {
                self.evaluator.initialize(world);
            }

            fn evaluate(&mut self, ctx: EvaluationCtx) -> Score {
                let score = self.evaluator.evaluate(ctx);
                self.curve.sample(score.get()).unwrap_or(Score::MIN)
            }
        }

        CurveEvaluator {
            curve,
            evaluator: self.into_evaluator(),
        }
    }

    /// Applies the given threshold to this evaluator's output score. If the
    /// output score is less than the threshold, the evaluator returns
    /// [`Score::MIN`].
    fn threshold(self, threshold: impl Into<Score>) -> impl Evaluator {
        struct OutputThresholdEvaluator<E: Evaluator> {
            threshold: Score,
            evaluator: E,
        }

        impl<E: Evaluator> Evaluator for OutputThresholdEvaluator<E> {
            fn name(&self) -> Cow<'static, str> {
                Cow::Owned(format!(
                    "{}.threshold({})",
                    self.evaluator.name(),
                    self.threshold,
                ))
            }

            fn initialize(&mut self, world: &mut World) {
                self.evaluator.initialize(world);
            }

            fn evaluate(&mut self, ctx: EvaluationCtx) -> Score {
                let score = self.evaluator.evaluate(ctx);
                if score < self.threshold {
                    Score::MIN
                } else {
                    score
                }
            }
        }

        OutputThresholdEvaluator {
            threshold: threshold.into(),
            evaluator: self.into_evaluator(),
        }
    }

    /// Labels this evaluator with the given [`ScoreLabel`].
    fn label(self, label: impl ScoreLabel) -> FlowNodeConfig {
        FlowNodeConfig::evaluator(self).label(label)
    }
}

/// All [`Evaluator`]s can be converted into themselves.
impl<E: Evaluator> IntoEvaluator<()> for E {
    type Evaluator = E;

    fn into_evaluator(self) -> Self::Evaluator {
        self
    }
}

/// [`World`] and [`Evaluation`] pair passed to [`Evaluator`]s.
#[derive(Clone, Debug)]
pub struct EvaluationCtx<'w> {
    /// The world in which the [`Evaluator`] is being evaluated.
    pub world: &'w World,
    /// The evaluation being performed.
    pub evaluation: Evaluation,
}

/// [`SystemInput`] type for [`Evaluator`] systems.
#[derive(Clone, PartialEq, Debug)]
pub struct Evaluation {
    /// The entity that is being scored.
    pub target: Entity,
}

impl SystemInput for Evaluation {
    type Param<'i> = Evaluation;
    type Inner<'i> = Evaluation;

    fn wrap(this: Self::Inner<'_>) -> Self::Param<'_> {
        this
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::{component::Component, entity::Entity, system::Resource, world::World};
    use bevy_hierarchy::BuildChildren;
    use bevy_math::curve::FunctionCurve;

    use crate::{
        aggregator::{sum, IntoAggregator},
        evaluator::{
            constant, parent, resource, target, Evaluation, EvaluationCtx, Evaluator, IntoEvaluator,
        },
        score::{Score, Scoreable},
    };

    #[derive(Resource)]
    struct TestResource(i32);

    impl Scoreable for TestResource {
        fn score(&self) -> Score {
            Score::new(self.0 as f32 / 100.)
        }
    }

    #[derive(Component)]
    struct TestComponent(i32);

    impl Scoreable for TestComponent {
        fn score(&self) -> Score {
            Score::new(self.0 as f32 / 100.)
        }
    }

    #[test]
    fn children_evaluator() {
        let mut world = World::new();

        let e1 = world.spawn(TestComponent(20)).id();
        let e2 = world.spawn(TestComponent(20)).id();
        let e3 = world.spawn(TestComponent(20)).id();
        let pent = world.spawn_empty().add_children(&[e1, e2, e3]).id();

        let mut evaluator = sum().score_children::<TestComponent>();
        evaluator.initialize(&mut world);

        let output = evaluator.evaluate(EvaluationCtx {
            world: &world,
            evaluation: Evaluation { target: pent },
        });

        assert_eq!(output, Score::new(0.6));
    }

    #[test]
    fn constant_evaluator() {
        let mut world = World::new();

        let mut evaluator = constant(0.8);
        evaluator.initialize(&mut world);

        let output = evaluator.evaluate(EvaluationCtx {
            world: &world,
            evaluation: Evaluation {
                target: Entity::PLACEHOLDER,
            },
        });

        assert_eq!(output, Score::new(0.8));
    }

    #[test]
    fn curve_evaluator() {
        let mut world = World::new();

        let mut evaluator =
            constant(0.5).curve(FunctionCurve::new(Score::INTERVAL, |x| Score::new(x * x)));
        evaluator.initialize(&mut world);

        let output = evaluator.evaluate(EvaluationCtx {
            world: &world,
            evaluation: Evaluation {
                target: Entity::PLACEHOLDER,
            },
        });

        assert_eq!(output, Score::new(0.25));
    }

    #[test]
    fn parent_evaluator() {
        let mut world = World::new();
        let ent = world.spawn_empty().id();
        world.spawn(TestComponent(25)).add_child(ent);

        let mut evaluator = parent::<TestComponent>();
        evaluator.initialize(&mut world);

        let output = evaluator.evaluate(EvaluationCtx {
            world: &world,
            evaluation: Evaluation { target: ent },
        });

        assert_eq!(output, Score::new(0.25));
    }

    #[test]
    fn resource_evaluator() {
        let mut world = World::new();
        world.insert_resource(TestResource(50));

        let mut evaluator = resource::<TestResource>();
        evaluator.initialize(&mut world);

        let output = evaluator.evaluate(EvaluationCtx {
            world: &world,
            evaluation: Evaluation {
                target: Entity::PLACEHOLDER,
            },
        });

        assert_eq!(output, Score::new(0.5));
    }

    #[test]
    fn target_evaluator() {
        let mut world = World::new();
        let entity = world.spawn(TestComponent(25)).id();

        let mut evaluator = target::<TestComponent>();
        evaluator.initialize(&mut world);

        let output = evaluator.evaluate(EvaluationCtx {
            world: &world,
            evaluation: Evaluation { target: entity },
        });

        assert_eq!(output, Score::new(0.25));
    }

    #[test]
    fn weight_evaluator() {
        let mut world = World::new();

        let mut evaluator = constant(0.8).weight(0.5);
        evaluator.initialize(&mut world);

        let output = evaluator.evaluate(EvaluationCtx {
            world: &world,
            evaluation: Evaluation {
                target: Entity::PLACEHOLDER,
            },
        });

        assert_eq!(output, Score::new(0.4));
    }
}
