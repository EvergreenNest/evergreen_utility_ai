use std::hint::black_box;

use bevy_ecs::{entity::Entity, world::World};
use criterion::{criterion_group, criterion_main, Bencher, Criterion};
use evergreen_utility_ai::{
    aggregator::{sum, IntoAggregator},
    evaluator::constant,
    flow::{FlowNodeConfig, WorldFlowExt},
};
use evergreen_utility_ai_macros::{FlowLabel, ScoreLabel};

#[derive(FlowLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct BenchFlow;

#[derive(ScoreLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct BenchScore(u8);

fn score(c: &mut Criterion) {
    c.bench_function("score/deep-3/thin/single", |b| {
        bench_scoring(b, 3, 1);
    });
    c.bench_function("score/deep-10/thin/single", |b| {
        bench_scoring(b, 10, 1);
    });
    c.bench_function("score/deep-25/thin/single", |b| {
        bench_scoring(b, 25, 1);
    });
    c.bench_function("score/deep-3/wide-15/single", |b| {
        bench_scoring(b, 3, 15);
    });
    c.bench_function("score/deep-10/wide-15/single", |b| {
        bench_scoring(b, 10, 15);
    });
    c.bench_function("score/deep-25/wide-15/single", |b| {
        bench_scoring(b, 25, 15);
    });
    c.bench_function("score/deep-3/wide-30/single", |b| {
        bench_scoring(b, 3, 30);
    });
    c.bench_function("score/deep-10/wide-30/single", |b| {
        bench_scoring(b, 10, 30);
    });
    c.bench_function("score/deep-25/wide-30/single", |b| {
        bench_scoring(b, 25, 30);
    });
}

fn bench_scoring(b: &mut Bencher, depth: usize, width: u8) {
    let mut world = World::new();
    for i in 0..width {
        world.add_nodes(BenchFlow, create_deep_node(depth).label(BenchScore(i)));
    }

    world.flow_scope(BenchFlow, |world, flow| {
        flow.initialize(world);
    });

    b.iter(|| {
        world.flow_scope(BenchFlow, |world, flow| {
            black_box(flow.run_readonly(world, Entity::PLACEHOLDER));
        });
    });
}

fn create_deep_node(depth: usize) -> FlowNodeConfig {
    let mut current = FlowNodeConfig::evaluator(constant(0.5));

    for _ in 1..depth {
        current = FlowNodeConfig::aggregator(sum(0.0).input_at_least(0.5), current);
    }

    current
}

criterion_group!(benches, score);
criterion_main!(benches);
