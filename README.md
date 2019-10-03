[![TravisCI](https://api.travis-ci.org/bugagashenkj/rust-salsa20.svg?branch=master)](https://travis-ci.org/bugagashenkj/rust-salsa20)
[![Crates.io](https://img.shields.io/crates/v/rust-salsa20.svg?)](https://crates.io/crates/rust-salsa20)
[![Docs](https://docs.rs/rust-salsa20/badge.svg)](https://docs.rs/rust-salsa20)

# Salsa20 stream cipher

[Salsa20](https://cr.yp.to/snuffle/spec.pdf) is a stream cipher built on a pseudo-random function based on add-rotate-xor operations — 32-bit addition, bitwise addition and rotation operations.

## Optimization of doubleround calculations

The doubleround function modifies the initial matrix 10 times to encrypt every 64-byte block. For each subsequent 64-byte block in the matrix, only 8 bytes associated with the counter are changed. Immutable bytes can be used to pre-compute the result of the first doubleround call. A preliminary calculation of the original matrix optimizes the first call to the doubleround function, reducing the number of calls to the quarterround function from 8 to 4 with a counter of less than 2 ^ 32 and from 8 to 6 with a counter of more than 2 ^ 32, which reduces the algorithmic complexity of the Salsa20 function by 5 and 2.5 percent respectively.

## Usage

To install rust-salsa20, add the following to your Cargo.toml:

```toml
[dependencies]
rust-salsa20 = "^0.2"
```

## Examples

### Generate
```rust
extern crate rust_salsa20;
use rust_salsa20::{Salsa20, Key::Key32};

fn main() {
    let key = Key32([
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
        17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31
    ]);
    let nonce = [1, 2, 3, 4, 5, 6, 7, 8];
    let mut salsa = Salsa20::new(key, nonce, 0);
    let mut buffer = [0; 10];
    salsa.generate(&mut buffer);

    assert_eq!(buffer, [45, 134, 38, 166, 142, 36, 28, 146, 116, 157]);
}
```

### Encrypt
```rust
extern crate rust_salsa20;
use rust_salsa20::{Salsa20, Key::Key32};

fn main() {
    let key = Key32([
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
        17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31
    ]);
    let nonce = [1, 2, 3, 4, 5, 6, 7, 8];
    let mut salsa = Salsa20::new(key, nonce, 0);
    let mut buffer = [1, 2, 3, 4, 5, 6, 7, 8, 9, 0];
    salsa.encrypt(&mut buffer);

    assert_eq!(buffer, [44, 132, 37, 162, 139, 34, 27, 154, 125, 157]);
}
```
## Contributors

See github for full [contributors list](https://github.com/bugagashenkj/rust-salsa20/graphs/contributors)
