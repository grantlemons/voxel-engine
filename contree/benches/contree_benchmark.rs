use contree::Contree;
use criterion::{Criterion, criterion_group, criterion_main};
use glam::Vec3;
use std::hint::black_box;

fn create_contree(size: u32, p: Vec3) -> Contree {
    assert!(size > 4, "The root node cannot be a leaf!");
    let mut contree = Contree {
        size,
        ..Default::default()
    };
    contree.insert(p, 10);
    contree
}

fn finding_benchmark(c: &mut Criterion) {
    let p = Vec3::ZERO;
    let mut contree = create_contree(4u32.pow(8), Vec3::splat(5.));

    c.bench_function("find empty", |b| b.iter(|| contree.find(black_box(p))));

    c.bench_function("insert", |b| {
        b.iter(|| contree.insert(black_box(p), black_box(0)))
    });
}

fn raycast_benchmark(c: &mut Criterion) {
    let contree = create_contree(16, Vec3::ZERO);

    fn fan(contree: &Contree) {
        let distance = 5.;
        let pos = Vec3::new(distance + 0.5, 0., 0.);

        let mut dir = Vec3::new(-distance, 1., 0.);
        while dir.y > -1. {
            contree.raycast(black_box(pos), black_box(dir));

            // round to account for FPE
            dir = ((dir + Vec3::new(0., -0.0005, 0.)) / 0.0005).round() * 0.0005;
        }
    }

    c.bench_function("single cast", |b| {
        b.iter(|| contree.raycast(Vec3::splat(50.), Vec3::splat(-1.)))
    });
    c.bench_function("fan", |b| b.iter(|| fan(black_box(&contree))));
}

criterion_group!(benches, finding_benchmark, raycast_benchmark);
criterion_main!(benches);
