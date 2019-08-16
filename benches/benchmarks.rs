#[macro_use]

extern crate criterion;
extern crate salsa20;

use salsa20::Salsa20;
use criterion::Criterion;
use criterion::black_box;

fn generate_1_kb(c: &mut Criterion) {
    let mut salsa20 = Salsa20::new(&[0; 16], &[0; 8], 0);
    let mut buffer = [0; 1024];

    c.bench_function("generate 1Kb", move |b| b.iter(|| {
        salsa20.generate(black_box(&mut buffer))
    }));
}

fn generate_1_kb_with_overflow(c: &mut Criterion) {
    let mut salsa20 = Salsa20::new(&[0; 16], &[0; 8], 0);
    let mut buffer = [0; 1024];

    c.bench_function("generate 1Kb with overflow", move |b| b.iter(|| {
        salsa20.generate(black_box(&mut buffer[0..7]));
        salsa20.generate(black_box(&mut buffer[7..259]));
        salsa20.generate(black_box(&mut buffer[259..938]));
        salsa20.generate(black_box(&mut buffer[938..1024]));
    }));
}

criterion_group!(
    benches,
    generate_1_kb,
    generate_1_kb_with_overflow
);
criterion_main!(benches);
