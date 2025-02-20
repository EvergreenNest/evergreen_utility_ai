//! Provides the [`Evaluator`] trait for evaluating target [`Entity`]s in a
//! [`World`].

use std::borrow::Cow;

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
pub trait IntoEvaluator<Marker> {
    /// The type of [`Evaluator`] that this value will be converted into.
    type Evaluator: Evaluator;

    /// Converts this value into a [`Evaluator`].
    fn into_evaluator(self) -> Self::Evaluator;

    /// Maps the output score of this evaluator using the given [`Mapper`].
    fn map<M>(self, mapper: impl IntoMapper<Score, M>) -> impl Evaluator
    where
        Self: Sized,
    {
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
                    "map({}, {})",
                    self.mapper.name(),
                    self.evaluator.name()
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
    fn invert(self) -> impl Evaluator
    where
        Self: Sized,
    {
        struct InvertEvaluator<P> {
            evaluator: P,
        }

        impl<P> Evaluator for InvertEvaluator<P>
        where
            P: Evaluator,
        {
            fn name(&self) -> Cow<'static, str> {
                Cow::Owned(format!("invert({})", self.evaluator.name()))
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
    fn weight(self, weight: impl Into<Score>) -> impl Evaluator
    where
        Self: Sized,
    {
        struct WeightEvaluator<P: Evaluator> {
            evaluator: P,
            weight: Score,
        }

        impl<P: Evaluator> Evaluator for WeightEvaluator<P> {
            fn name(&self) -> Cow<'static, str> {
                Cow::Owned(format!(
                    "weight({}, {})",
                    self.evaluator.name(),
                    self.weight
                ))
            }

            fn initialize(&mut self, world: &mut World) {
                self.evaluator.initialize(world);
            }

            fn evaluate(&mut self, ctx: EvaluationCtx) -> Score {
                self.evaluator.evaluate(ctx) * self.weight
            }
        }

        WeightEvaluator {
            evaluator: self.into_evaluator(),
            weight: weight.into(),
        }
    }

    /// Applies the given [`Curve`] to this evaluator's output score. If the
    /// curve cannot be sampled at the output score value, the evaluator returns
    /// [`Score::MIN`].
    fn curve(self, curve: impl Curve<Score> + Send + Sync + 'static) -> impl Evaluator
    where
        Self: Sized,
    {
        struct CurveEvaluator<C: Curve<Score> + Send + Sync + 'static, P: Evaluator> {
            curve: C,
            evaluator: P,
        }

        impl<C: Curve<Score> + Send + Sync + 'static, P: Evaluator> Evaluator for CurveEvaluator<C, P> {
            fn name(&self) -> Cow<'static, str> {
                Cow::Owned(format!(
                    "curve({}, {})",
                    std::any::type_name::<C>(),
                    self.evaluator.name()
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

    /// Labels this evaluator with the given [`ScoreLabel`].
    fn label(self, label: impl ScoreLabel) -> FlowNodeConfig
    where
        Self: Sized,
    {
        FlowNodeConfig::evaluator(self).label(label)
    }
}

/// All [`Evaluator`]s can be converted into themselves.
impl<P: Evaluator> IntoEvaluator<()> for P {
    type Evaluator = P;

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

        let mut evaluator = sum(0.0).score_children::<TestComponent>();
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
