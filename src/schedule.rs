use alloc::boxed::Box;

use bevy_ecs::{
    component::Tick,
    schedule::{
        graph::Direction,
        traits::{GraphNode, GraphNodeId, ProcessedConfigs, ScheduleExecutable, ScheduleGraph},
        InfallibleReadOnlySystem, NodeConfig, NodeConfigs, Schedule, ScheduleBuildPass,
        ScheduleExecutor,
    },
    world::World,
};
use evergreen_utility_ai_macros::ScoreLabel;
use thiserror::Error;

use crate::{
    aggregator::Aggregation, evaluator::Evaluation, label::InternedScoreLabel, score::Score,
};

#[derive(Default)]
pub struct ScoringGraph {
    changed: bool,
}

impl ScheduleGraph for ScoringGraph {
    type Id = ScoringNodeId;
    type Executable = ScoringExecutable;
    type BuildError = ScoringBuildError;
    type BuildSettings = ();
    type ExecutorKind = ();
    type GlobalMetadata = ();

    fn new_executor((): Self::ExecutorKind) -> Box<dyn ScheduleExecutor<Self>> {
        todo!()
    }

    fn changed(&self) -> bool {
        self.changed
    }

    fn set_changed(&mut self, changed: bool) {
        self.changed = changed;
    }

    fn add_build_pass<P: ScheduleBuildPass<Self>>(&mut self, _pass: P) {
        todo!()
    }

    fn remove_build_pass<P: ScheduleBuildPass<Self>>(&mut self) {
        todo!()
    }

    fn get_build_settings(&self) -> &Self::BuildSettings {
        &()
    }

    fn set_build_settings(&mut self, (): Self::BuildSettings) {}

    fn initialize(&mut self, world: &mut World) {
        todo!()
    }

    fn update(
        &mut self,
        world: &mut World,
        executable: &mut Self::Executable,
        global_metadata: &Self::GlobalMetadata,
        label: bevy_ecs::schedule::InternedScheduleLabel,
    ) -> Result<(), Self::BuildError> {
        todo!()
    }
}

#[derive(Default)]
pub struct ScoringExecutable {}

impl ScheduleExecutable for ScoringExecutable {
    fn apply_deferred(&mut self, _world: &mut World) {
        todo!()
    }

    fn check_change_ticks(&mut self, _change_tick: Tick) {
        todo!()
    }
}

#[derive(Error, Debug)]
pub enum ScoringBuildError {}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum ScoringNodeId {
    Evaluator(usize),
    Aggregator(usize),
}

impl GraphNodeId for ScoringNodeId {
    type Pair = (Self, Self);
    type Directed = (Self, Direction);
}

pub type AggregatorSystem = InfallibleReadOnlySystem<Aggregation, Score>;

impl GraphNode<ScoringGraph> for AggregatorSystem {
    type Metadata = ();
    type GroupMetadata = ();
    type ProcessData = ();

    fn into_config(self) -> NodeConfig<Self, ScoringGraph> {
        todo!()
    }

    fn process_config(
        graph: &mut ScoringGraph,
        config: NodeConfig<Self, ScoringGraph>,
    ) -> Result<ScoringNodeId, ScoringBuildError> {
        todo!()
    }

    fn process_configs(
        graph: &mut ScoringGraph,
        configs: NodeConfigs<Self, ScoringGraph>,
        collect_nodes: bool,
    ) -> Result<ProcessedConfigs<Self, ScoringGraph>, ScoringBuildError> {
        todo!()
    }
}

pub type EvaluatorSystem = InfallibleReadOnlySystem<Evaluation, Score>;

impl GraphNode<ScoringGraph> for EvaluatorSystem {
    type Metadata = ();
    type GroupMetadata = ();
    type ProcessData = ();

    fn into_config(self) -> NodeConfig<Self, ScoringGraph> {
        todo!()
    }

    fn process_config(
        graph: &mut ScoringGraph,
        config: NodeConfig<Self, ScoringGraph>,
    ) -> Result<ScoringNodeId, ScoringBuildError> {
        todo!()
    }

    fn process_configs(
        graph: &mut ScoringGraph,
        configs: NodeConfigs<Self, ScoringGraph>,
        collect_nodes: bool,
    ) -> Result<ProcessedConfigs<Self, ScoringGraph>, ScoringBuildError> {
        todo!()
    }
}

impl GraphNode<ScoringGraph> for InternedScoreLabel {
    type Metadata = ();
    type GroupMetadata = ();
    type ProcessData = ();

    fn into_config(self) -> NodeConfig<Self, ScoringGraph> {
        todo!()
    }

    fn process_config(
        graph: &mut ScoringGraph,
        config: NodeConfig<Self, ScoringGraph>,
    ) -> Result<ScoringNodeId, ScoringBuildError> {
        todo!()
    }

    fn process_configs(
        graph: &mut ScoringGraph,
        configs: NodeConfigs<Self, ScoringGraph>,
        collect_nodes: bool,
    ) -> Result<ProcessedConfigs<Self, ScoringGraph>, ScoringBuildError> {
        todo!()
    }
}

fn test(mut t: Schedule<ScoringGraph>) {
    use crate as evergreen_utility_ai;

    #[derive(ScoreLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
    struct TestScore;

    t.process_nodes(TestScore);
}
