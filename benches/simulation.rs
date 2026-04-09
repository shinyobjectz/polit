use criterion::{criterion_group, criterion_main, Criterion};

fn bench_placeholder(c: &mut Criterion) {
    c.bench_function("placeholder", |b| {
        b.iter(|| {
            // Will be replaced with real simulation benchmarks
            let x: u64 = (1..100).sum();
            criterion::black_box(x);
        })
    });
}

criterion_group!(benches, bench_placeholder);
criterion_main!(benches);
