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

fn quarterround(data: &mut [u32; 16], [y0, y1, y2, y3]: [usize; 4]) {
    data[y1] ^= data[y0].wrapping_add(data[y3]).rotate_left(7);
    data[y2] ^= data[y1].wrapping_add(data[y0]).rotate_left(9);
    data[y3] ^= data[y2].wrapping_add(data[y1]).rotate_left(13);
    data[y0] ^= data[y3].wrapping_add(data[y2]).rotate_left(18);
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
    fn salsa20_test() {
        let mut input_data = [0; 64];
        let mut expected_data = [0; 64];

        salsa20(&mut input_data);
        assert_eq!(input_data.to_vec(), expected_data.to_vec());

        input_data = [
            88, 118, 104, 54, 79, 201, 235, 79, 3, 81, 156, 47, 203, 26, 244,
            243, 191, 187, 234, 136, 211, 159, 13, 115, 76, 55, 82, 183, 3,
            117, 222, 37, 86, 16, 179, 207, 49, 237, 179, 48, 1, 106, 178, 219,
            175, 199, 166, 48, 238, 55, 204, 36, 31, 240, 32, 63, 15, 83, 93,
            161, 116, 147, 48, 113
        ];

        expected_data = [
            179, 19, 48, 202, 219, 236, 232, 135, 111, 155, 110, 18, 24, 232,
            95, 158, 26, 110, 170, 154, 109, 42, 178, 168, 156, 240, 248, 238,
            168, 196, 190, 203, 69, 144, 51, 57, 29, 29, 150, 26, 150, 30, 235,
            249, 190, 163, 251, 48, 27, 111, 114, 114, 118, 40, 152, 157, 180,
            57, 27, 94, 107, 42, 236, 35
        ];

        salsa20(&mut input_data);
        assert_eq!(input_data.to_vec(), expected_data.to_vec());
    }
}
