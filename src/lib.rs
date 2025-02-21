//! A utility AI library for [Bevy Engine](https://bevyengine.org/).
//!
//! # Key Concepts
//!
//! - [`Score`]: An `f32` between 0 and 1.
//! - [`Evaluator`]: Returns a score based on the world state and the target entity.
//! - [`Aggregator`]: Aggregates scores from multiple evaluators and/or other aggregators.
//!   Also provided with the world state and the target entity.
//! - [`Flow`]: A graph of aggregators and evaluators that can be evaluated to get scores.
//!   This is architected in a similar fashion to bevy schedules.
//! - [`Selector`]: Selects an action based on the computed scores from a previously run flow.
//!
//! Add evaluators and aggregators to flows, then run the flow to
//! get scores.
//!
//! ## [`Flow`] vs [`Schedule`]
//!
//! Flows are similar to bevy schedules, however there are a few key differences:
//! - Rather than storing systems, flows store aggregators and
//!   evaluators (which themselves *can be* systems).
//! - There are no system sets.
//! - All world access in aggregators and evaluators is read-only. This
//!   helps enforce idempotency within the scoring phase.
//! - There is no [`ApplyDeferred`] system that run between and after all systems.
//!   Because all world access is read-only, no commands can be queued.
//!
//! [`Score`]: crate::score::Score
//! [`Evaluator`]: crate::evaluator::Evaluator
//! [`Aggregator`]: crate::aggregator::Aggregator
//! [`Flow`]: crate::flow::Flow
//! [`Selector`]: crate::selector::Selector
//! [`Schedule`]: bevy_ecs::schedule::Schedule
//! [`System`]: bevy_ecs::system::System
//! [`ApplyDeferred`]: bevy_ecs::schedule::apply_deferred

#![warn(missing_docs)]

pub mod aggregator;
pub mod command;
pub mod component;
pub mod evaluator;
pub mod flow;
pub mod label;
pub mod mapper;
pub mod score;
pub mod selector;

pub use evergreen_utility_ai_macros as macros;

#[cfg(test)]
mod tests {

    use bevy_ecs::{component::Component, world::World};
    use bevy_hierarchy::BuildChildren;
    use bevy_math::{curve::FunctionCurve, ops::powf};
    use bevy_time::Time;

    use crate::{
        self as evergreen_utility_ai,
        aggregator::{sum, IntoAggregator},
        evaluator::{constant, parent, resource, target, IntoEvaluator},
        flow::WorldFlowExt,
        label::ScoreLabel as _,
        macros::{FlowLabel, ScoreLabel},
        mapper::Mapping,
        score::{Score, Scoreable},
    };

    #[derive(FlowLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
    pub struct TestFlow;

    #[derive(ScoreLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
    pub struct HealthScore;

    #[derive(ScoreLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
    pub struct TotalHealthScore;

    #[derive(ScoreLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
    pub struct FuelScore;

    #[derive(ScoreLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
    pub struct TimeScore;

    #[derive(Default)]
    pub struct GameTime;

    impl Scoreable for Time<GameTime> {
        fn score(&self) -> Score {
            Score::new(self.elapsed_secs_wrapped() / self.wrap_period().as_secs_f32())
        }
    }

    #[derive(Component)]
    pub struct Health(i32);

    impl Scoreable for Health {
        fn score(&self) -> Score {
            Score::new(self.0 as f32 / 100.)
        }
    }

    #[derive(Component)]
    pub struct Fuel(i32);

    impl Scoreable for Fuel {
        fn score(&self) -> Score {
            Score::new(self.0 as f32 / 100.)
        }
    }

    #[test]
    fn evaluator_test() {
        let mut world = World::new();

        world.add_nodes(TestFlow, target::<Health>().label(HealthScore));

        let npc = world.spawn(Health(50)).id();
        let scores = world.run_flow(TestFlow, npc);
        assert_eq!(scores.get(&HealthScore.intern()), Some(&Score::new(0.5)));
    }

    #[test]
    fn aggregator_test() {
        let mut world = World::new();

        world.add_nodes(
            TestFlow,
            sum()
                .with_children((target::<Health>(), constant(0.25)))
                .label(HealthScore),
        );

        let npc = world.spawn(Health(50)).id();
        let scores = world.run_flow(TestFlow, npc);
        assert_eq!(scores.get(&HealthScore.intern()), Some(&Score::new(0.75)));
    }

    #[test]
    fn flow_test() {
        let mut world = World::new();
        world.insert_resource(Time::<GameTime>::default());

        world.add_nodes(
            TestFlow,
            (
                sum()
                    .threshold(0.2)
                    .curve(FunctionCurve::new(Score::INTERVAL, |x| {
                        Score::new(powf(x, 3.))
                    }))
                    .with_children((
                        sum().threshold(0.2).score_children::<Health>(),
                        target::<Health>().label(HealthScore),
                    ))
                    .label(TotalHealthScore),
                parent::<Fuel>().weight(0.25).label(FuelScore),
                resource::<Time<GameTime>>().label(TimeScore),
                constant(0.5).map(|mapping: Mapping<Score>| mapping.value * 0.25), // This doesn't have a label nor parent node, so it won't actually get added.
            ),
        );

        let gun = world.spawn(Health(50)).id();
        let npc = world.spawn(Health(100)).add_child(gun).id();
        let _car = world.spawn(Fuel(50)).add_child(npc).id();

        let scores = world.run_flow(TestFlow, npc);
        assert_eq!(scores.get(&HealthScore.intern()), Some(&Score::new(1.)));
    }
}
