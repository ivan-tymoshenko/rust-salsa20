extern crate criterion;
extern crate rust_salsa20;

use rust_salsa20::{Salsa20, Key::Key16};
use criterion::*;

fn encrypt_1_kb(c: &mut Criterion) {
    let mut salsa20 = Salsa20::new(&Key16([1; 16]), &[0; 8], 0);
    let mut buffer = [0; 1024];

    c.bench(
        "encrypt",
        Benchmark::new(
            "1Kb", move |b| b.iter(|| salsa20.encrypt(black_box(&mut buffer)))
        ).throughput(Throughput::Bytes(1024))
    );
}

fn generate_1_kb(c: &mut Criterion) {
    let mut salsa20 = Salsa20::new(&Key16([2; 16]), &[0; 8], 0);
    let mut buffer = [0; 1024];

    c.bench(
        "generate",
        Benchmark::new(
            "1Kb", move |b| b.iter(|| salsa20.generate(black_box(&mut buffer)))
        ).throughput(Throughput::Bytes(1024))
    );
}

fn generate_1_kb_with_overflow(c: &mut Criterion) {
    let mut salsa20 = Salsa20::new(&Key16([3; 16]), &[0; 8], 0);
    let mut buffer = [0; 1024];

    c.bench(
        "generate with overflow",
        Benchmark::new(
            "1Kb",
            move |b| b.iter(|| {
                salsa20.generate(black_box(&mut buffer[0..7]));
                salsa20.generate(black_box(&mut buffer[7..259]));
                salsa20.generate(black_box(&mut buffer[259..938]));
                salsa20.generate(black_box(&mut buffer[938..1024]));
            })
        ).throughput(Throughput::Bytes(1024))
    );
}

criterion_group!(
    benches,
    encrypt_1_kb,
    generate_1_kb,
    generate_1_kb_with_overflow
);
criterion_main!(benches);
