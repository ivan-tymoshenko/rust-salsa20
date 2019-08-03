use std::mem::transmute;

const CONSTS_16: [[u8; 4]; 4] = [
    [101, 120, 112, 97],
    [110, 100, 32, 49],
    [54, 45, 98, 121],
    [116, 101, 32, 107]
];

const CONSTS_32: [[u8; 4]; 4] = [
    [101, 120, 112, 97],
    [110, 100, 32, 51],
    [50, 45, 98, 121],
    [116, 101, 32, 107]
];

fn rotl(value: u32, shift: u8) -> u32 {
    (value << shift) | (value >> (32 - shift))
} 

fn quarterround(data: &mut [u32; 16], [y0, y1, y2, y3]: [usize; 4]) {
    data[y1] ^= rotl(data[y0].wrapping_add(data[y3]), 7);
    data[y2] ^= rotl(data[y1].wrapping_add(data[y0]), 9);
    data[y3] ^= rotl(data[y2].wrapping_add(data[y1]), 13);
    data[y0] ^= rotl(data[y3].wrapping_add(data[y2]), 18);
}

fn rowround(data: &mut [u32; 16]) {
    quarterround(data, [0, 1, 2, 3]);
    quarterround(data, [5, 6, 7, 4]);
    quarterround(data, [10, 11, 8, 9]);
    quarterround(data, [15, 12, 13, 14]);
}

fn columround(data: &mut [u32; 16]) {
    quarterround(data, [0, 4, 8, 12]);
    quarterround(data, [5, 9, 13, 1]);
    quarterround(data, [10, 14, 2, 6]);
    quarterround(data, [15, 3, 7, 11]);
}

fn doubleround(data: &mut [u32; 16]) {
    columround(data);
    rowround(data);
}

fn bytes_to_word(data: [u8; 4]) -> u32 {
    unsafe { transmute::<[u8; 4], u32>(data).to_le() }
}

fn word_to_bytes(value: u32) -> [u8; 4] {
    unsafe { transmute::<u32, [u8; 4]>(value.to_le()) }
}

fn salsa20(data: &mut [u8; 64]) {
    let mut words = [0; 16];

    for word_index in 0..16 {
        let byte_index = word_index * 4;
        let word = bytes_to_word([
            data[byte_index],
            data[byte_index + 1],
            data[byte_index + 2],
            data[byte_index + 3]
        ]);
        words[word_index] = word;
    }

    let words_copy = words;

    for _ in 0..10 {
        doubleround(&mut words);
    }

    for word_index in 0..16 {
        let byte_index = word_index * 4;
        let sum = words_copy[word_index].wrapping_add(words[word_index]);
        let bytes = word_to_bytes(sum);
        data[byte_index..byte_index + 4].copy_from_slice(&bytes[..]);
    }
}

fn expand16(key: &[u8; 16], nonce: &[u8; 16], keystream: &mut [u8; 64]) {
    for i in 0..4 {
        for j in 0..4 {
            keystream[i * 20 + j] = CONSTS_16[i][j];
        }
    }

    for i in 0..16 {
        keystream[i + 4] = key[i];
        keystream[i + 44] = key[i];
        keystream[i + 24] = nonce[i];
    }

    salsa20(keystream);
}

fn expand32(key: &[u8; 32], nonce: &[u8; 16], keystream: &mut [u8; 64]) {
    for i in 0..4 {
        for j in 0..4 {
            keystream[i * 20 + j] = CONSTS_32[i][j];
        }
    }

    for i in 0..16 {
        keystream[i + 4] = key[i];
        keystream[i + 44] = key[i + 16];
        keystream[i + 24] = nonce[i];
    }

    salsa20(keystream);
}

pub enum Key<'a> {
    Key16(&'a [u8; 16]),
    Key32(&'a [u8; 32])
}

impl<'a> Key<'a> {
    fn expand(&self, nonce: &[u8; 16], keystream: &mut [u8; 64]) {
        match self {
            Key::Key16(key) => expand16(key, nonce, keystream),
            Key::Key32(key) => expand32(key, nonce, keystream)
        }
    }
}

pub fn encrypt(key: Key, nonce: &[u8; 8], buffer: &mut [u8]) {
    let mut keystream: [u8; 64] = [0; 64];
    let mut n: [u8; 16] = [0; 16];

    n[..8].copy_from_slice(&nonce[..]);

    for byte_index in 0..buffer.len() {
        if byte_index % 64 == 0 {
            let bytes = word_to_bytes((byte_index / 64) as u32);
            n[8..12].copy_from_slice(&bytes[..]);
            key.expand(&n, &mut keystream);
        }
        buffer[byte_index] ^= keystream[byte_index % 64];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotl_test() {
        assert_eq!(rotl(22, 3), 176);
    }
}
