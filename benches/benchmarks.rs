#[macro_use]

extern crate criterion;
extern crate salsa20;

use salsa20::Salsa20;
use criterion::Criterion;
use criterion::black_box;

fn generate_64b(c: &mut Criterion) {
    let mut salsa20 = Salsa20::new(&[0; 16], &[0; 8], 0);
    let mut buffer = [0; 64];

    c.bench_function("generate 64b", move |b| b.iter(|| {
        salsa20.generate(black_box(&mut buffer))
    }));
}

fn generate_1Kb(c: &mut Criterion) {
    let mut salsa20 = Salsa20::new(&[0; 16], &[0; 8], 0);
    let mut buffer = [0; 1024];

    c.bench_function("generate 1Kb", move |b| b.iter(|| {
        salsa20.generate(black_box(&mut buffer))
    }));
}

fn generate_1Mb(c: &mut Criterion) {
    let mut salsa20 = Salsa20::new(&[0; 16], &[0; 8], 0);
    let mut buffer = [0; 1024 * 1024];

    c.bench_function("generate 1Mb", move |b| b.iter(|| {
        salsa20.generate(black_box(&mut buffer))
    }));
}

criterion_group!(
    benches,
    generate_64b,
    generate_1Kb,
    generate_1Mb,
);
criterion_main!(benches);
