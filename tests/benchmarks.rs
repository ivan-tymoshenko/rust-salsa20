extern crate salsa20;
extern crate rand;

use std::time::Instant;
use salsa20::Salsa20;

fn new_test_salsa20() -> Salsa20 {
    let key: [u8; 16] = rand::random();
    let nonce: [u8; 8] = rand::random();
    Salsa20::new(&key, &nonce, 0)
}

#[test]
fn generate_bench() {
    let mut salsa20 = new_test_salsa20();
    let mut buffer = [0; 1024 * 1024];

    let now = Instant::now();
    for _ in 0..10 {
        salsa20.generate(&mut buffer);
    }
    let time = now.elapsed().as_millis();
    println!("Millisec: {}", time);
    println!("Mb/s: {}", 10_000 / time);
}
