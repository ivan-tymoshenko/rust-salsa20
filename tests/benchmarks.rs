extern crate salsa20;

use salsa20::Salsa20;

#[test]
#[inline(never)]
fn generate_bench() {
    let mut salsa20 = Salsa20::new(&[0; 16], &[0; 8], 0);
    let mut buffer = [0; 1024 * 1024];

    let n = 1000;
    let mut times = Vec::with_capacity(n);
    for _ in 0..n {
        let start = std::time::Instant::now();
        salsa20.generate(&mut buffer);
        times.push(start.elapsed());
    }

    let min_time = times.into_iter().min().unwrap();
    let speed = 1_000_000_000 / min_time.as_nanos();
    println!("Time: {:?}", min_time);
    println!("Speed: {} Mb/s", speed);
}
