extern crate salsa20;
extern crate rand;

use salsa20::Salsa20;

#[test]
#[inline(never)]
fn generate_bench() {
    let key: [u8; 16] = rand::random();
    let nonce: [u8; 8] = rand::random();

    let mut salsa20 = Salsa20::new(&key, &nonce, 0);
    let mut buffer = [0; 1024];

    let n = 500;
    let mut times = Vec::with_capacity(n);
    for _ in 0..n {
        let start = std::time::Instant::now();
        salsa20.generate(&mut buffer);
        times.push(start.elapsed());
    }

    let min_time = times.into_iter().min().unwrap();
    let speed = 1_000_000_000 / (min_time.as_nanos() * 1024);
    println!("Time: {:?}", min_time);
    println!("Speed: {} Mb/s", speed);
}
