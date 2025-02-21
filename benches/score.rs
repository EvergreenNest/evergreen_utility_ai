use std::hint::black_box;

use bevy::{app::App, MinimalPlugins};
use bevy_ecs::{
    entity::Entity,
    system::{IntoSystem, System},
    world::World,
};
use criterion::{criterion_group, criterion_main, Bencher, Criterion};
use evergreen_utility_ai::{
    aggregator::{sum, IntoAggregator},
    component::{run_all_entity_flows, EntityFlow},
    evaluator::constant,
    flow::{FlowNodeConfig, WorldFlowExt},
};
use evergreen_utility_ai_macros::{FlowLabel, ScoreLabel};

#[derive(FlowLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct BenchFlow;

#[derive(ScoreLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct BenchScore(u8);

fn run_entity_flows(c: &mut Criterion) {
    c.bench_function("run_all_entity_flows/count-100/deep-1/wide-30", |b| {
        bench_run_all_entity_flows(b, 100, 3, 30);
    });
    c.bench_function("run_all_entity_flows/count-100/deep-3/wide-5", |b| {
        bench_run_all_entity_flows(b, 100, 3, 5);
    });
    c.bench_function("run_all_entity_flows/count-100/deep-3/wide-15", |b| {
        bench_run_all_entity_flows(b, 100, 3, 15);
    });
    c.bench_function("run_all_entity_flows/count-100/deep-3/wide-30", |b| {
        bench_run_all_entity_flows(b, 100, 3, 30);
    });
    c.bench_function("run_all_entity_flows/count-10000/deep-1/wide-30", |b| {
        bench_run_all_entity_flows(b, 10_000, 3, 30);
    });
    c.bench_function("run_all_entity_flows/count-10000/deep-3/wide-5", |b| {
        bench_run_all_entity_flows(b, 10_000, 3, 5);
    });
    c.bench_function("run_all_entity_flows/count-10000/deep-3/wide-15", |b| {
        bench_run_all_entity_flows(b, 10_000, 3, 15);
    });
    c.bench_function("run_all_entity_flows/count-10000/deep-3/wide-30", |b| {
        bench_run_all_entity_flows(b, 10_000, 3, 30);
    });
    c.bench_function("run_all_entity_flows/count-1000000/deep-1/wide-30", |b| {
        bench_run_all_entity_flows(b, 1_000_000, 1, 30);
    });
    c.bench_function("run_all_entity_flows/count-1000000/deep-3/wide-5", |b| {
        bench_run_all_entity_flows(b, 1_000_000, 3, 5);
    });
    c.bench_function("run_all_entity_flows/count-1000000/deep-3/wide-15", |b| {
        bench_run_all_entity_flows(b, 1_000_000, 3, 15);
    });
    c.bench_function("run_all_entity_flows/count-1000000/deep-3/wide-30", |b| {
        bench_run_all_entity_flows(b, 1_000_000, 3, 30);
    });
}

fn bench_run_all_entity_flows(b: &mut Bencher, entities: usize, depth: usize, width: u8) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);

    let world = app.world_mut();

    for i in 0..width {
        world.add_nodes(BenchFlow, create_deep_node(depth).label(BenchScore(i)));
    }

    world.flow_scope(BenchFlow, |world, flow| {
        flow.initialize(world);
    });

    for _ in 0..entities {
        world.spawn(EntityFlow::new(BenchFlow));
    }
    world.flush();

    let mut run_all_entity_flows = IntoSystem::into_system(run_all_entity_flows);
    run_all_entity_flows.initialize(world);

    b.iter(|| {
        run_all_entity_flows.run((), world);
    });
}

fn run_flow(c: &mut Criterion) {
    c.bench_function("run_flow/deep-3/thin", |b| {
        bench_run_flow(b, 3, 1);
    });
    c.bench_function("run_flow/deep-10/thin", |b| {
        bench_run_flow(b, 10, 1);
    });
    c.bench_function("run_flow/deep-25/thin", |b| {
        bench_run_flow(b, 25, 1);
    });
    c.bench_function("run_flow/deep-3/wide-15", |b| {
        bench_run_flow(b, 3, 15);
    });
    c.bench_function("run_flow/deep-10/wide-15", |b| {
        bench_run_flow(b, 10, 15);
    });
    c.bench_function("run_flow/deep-25/wide-15", |b| {
        bench_run_flow(b, 25, 15);
    });
    c.bench_function("run_flow/deep-3/wide-30", |b| {
        bench_run_flow(b, 3, 30);
    });
    c.bench_function("run_flow/deep-10/wide-30", |b| {
        bench_run_flow(b, 10, 30);
    });
    c.bench_function("run_flow/deep-25/wide-30", |b| {
        bench_run_flow(b, 25, 30);
    });
}

fn bench_run_flow(b: &mut Bencher, depth: usize, width: u8) {
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
        current = FlowNodeConfig::aggregator(sum().input_threshold(0.5), current);
    }

    current
}

criterion_group!(benches, run_flow, run_entity_flows);
criterion_main!(benches);
