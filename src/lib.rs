#![no_std]

fn quarterround(y0: u32, y1: u32, y2: u32, y3: u32) -> [u32; 4] {
    let y1 = y1 ^ y0.wrapping_add(y3).rotate_left(7);
    let y2 = y2 ^ y1.wrapping_add(y0).rotate_left(9);
    let y3 = y3 ^ y2.wrapping_add(y1).rotate_left(13);
    let y0 = y0 ^ y3.wrapping_add(y2).rotate_left(18);

    [y0, y1, y2, y3]
}

fn doubleround(y: [u32; 16]) -> [u32; 16] {
    // columnround
    let [
        [z0, z4, z8, z12],
        [z5, z9, z13, z1],
        [z10, z14, z2, z6],
        [z15, z3, z7, z11]
    ] = [
        quarterround(y[0], y[4], y[8], y[12]),
        quarterround(y[5], y[9], y[13], y[1]),
        quarterround(y[10], y[14], y[2], y[6]),
        quarterround(y[15], y[3], y[7], y[11]),
    ];
    
    // rowround
    let [
        [z0, z1, z2, z3],
        [z5, z6, z7, z4],
        [z10, z11, z8, z9],
        [z15, z12, z13, z14]
    ] = [
        quarterround(z0, z1, z2, z3),
        quarterround(z5, z6, z7, z4),
        quarterround(z10, z11, z8, z9),
        quarterround(z15, z12, z13, z14)
    ];

    [z0, z1, z2, z3, z4, z5, z6, z7, z8, z9, z10, z11, z12, z13, z14, z15]
}

fn doubleround_10(data: [u32; 16]) -> [u32; 16] {
    let mut y = data;

    for _ in 0..10 {
        y = doubleround(y);
    }
    y
}

fn core_generate(data: &[u32; 16], hash: &mut[u8]) {
    let data_copy = doubleround_10(data.clone());

    data.iter()
        .zip(data_copy.iter())
        .enumerate()
        .for_each(|(index, (value, &value_copy))| {
            let offset = index * 4;
            let sum = value.wrapping_add(value_copy); 
            hash[offset..offset + 4].copy_from_slice(&sum.to_le_bytes());
        });
}

fn core_encrypt(data: &[u32; 16], hash: &mut[u8]) {
    let data_copy = doubleround_10(data.clone());

    data.iter()
        .zip(data_copy.iter())
        .enumerate()
        .for_each(|(index, (value, &value_copy))| {
            let offset = index * 4;
            let sum = value.wrapping_add(value_copy); 
            xor_from_slice(&mut hash[offset..offset + 4], &sum.to_le_bytes());
        });
}

fn u8_to_u32(value: &[u8], buffer: &mut [u32]) {
    for (index, word) in buffer.iter_mut().enumerate() {
        let offset = index * 4;
        *word = u32::from_le_bytes([
            value[offset],
            value[offset + 1],
            value[offset + 2],
            value[offset + 3]
        ]);
    }
}

fn xor_from_slice(to: &mut [u8], from: &[u8]) {
    for (to_byte, from_byte) in to.iter_mut().zip(from.iter()) {
        *to_byte ^= *from_byte;
    }
}

#[derive(Clone, Copy)]
struct Overflow {
    buffer: [u8; 64],
    offset: usize
}

impl Overflow {
    fn new() -> Overflow {
        Overflow { buffer: [0; 64], offset: 64 }
    }

    fn copy_to(&mut self, buffer: &mut [u8]) {
        let offset = self.offset;
        self.offset += buffer.len();
        buffer[..].copy_from_slice(&self.buffer[offset..self.offset]);
    }

    fn xor_to(&mut self, buffer: &mut [u8]) {
        let offset = self.offset;
        self.offset += buffer.len();
        xor_from_slice(&mut buffer[..], &self.buffer[offset..self.offset]);
    }
}

#[derive(Clone, Copy)]
pub struct Salsa20 {
    counter: u64,
    block: [u32; 16],
    overflow: Overflow 
}

impl Salsa20 {
    pub fn new(key: &[u8], nonce: &[u8; 8], counter: u64) -> Salsa20 {
        let mut block = [0; 16];
        block[0] = 1634760805;
        block[15] = 1797285236;
        u8_to_u32(&nonce[..], &mut block[6..8]);
        u8_to_u32(&counter.to_le_bytes(), &mut block[8..10]);

        match key.len() {
            16 => {
                u8_to_u32(&key[..], &mut block[1..5]);
                u8_to_u32(&key[..], &mut block[11..15]);
                block[5] = 824206446;
                block[10] = 1885482294;
            }
            32 => {
                u8_to_u32(&key[0..16], &mut block[1..5]);
                u8_to_u32(&key[16..32], &mut block[11..15]);
                block[5] = 824206446;
                block[10] = 2036477238;
            } _ => {
                panic!("Wrong key size.");
            }
        }

        Salsa20 { block, counter, overflow: Overflow::new() }
    }

    fn inc_counter(&mut self) {
        self.counter.wrapping_add(1);
        u8_to_u32(&self.counter.to_le_bytes(), &mut self.block[8..10]);
    }

    pub fn generate(&mut self, buffer: &mut [u8]) {
        let buffer_len = buffer.len();
        let overflow_len = 64 - self.overflow.offset;

        if overflow_len != 0 {
            if buffer_len >= overflow_len {
                self.overflow.copy_to(&mut buffer[..overflow_len]);
            } else {
                self.overflow.copy_to(&mut buffer[..]);
                return;
            }
        }

        let last_offset = buffer_len - (buffer_len - overflow_len) % 64; 

        for offset in (overflow_len..last_offset).step_by(64) {
            core_generate(&self.block, &mut buffer[offset..offset + 64]);
            self.inc_counter();
        }

        if last_offset != buffer_len {
            core_generate(&self.block, &mut self.overflow.buffer);
            self.inc_counter();
            self.overflow.offset = 0;
            self.overflow.copy_to(&mut buffer[last_offset..]);
        }
    }

    pub fn encrypt(&mut self, buffer: &mut [u8]) {
        let buffer_len = buffer.len();
        let overflow_len = 64 - self.overflow.offset;

        if overflow_len != 0 {
            if buffer_len >= overflow_len {
                self.overflow.xor_to(&mut buffer[..overflow_len]);
            } else {
                self.overflow.xor_to(&mut buffer[..]);
                return;
            }
        }

        let last_offset = buffer_len - (buffer_len - overflow_len) % 64; 

        for offset in (overflow_len..last_offset).step_by(64) {
            core_encrypt(&self.block, &mut buffer[offset..offset + 64]);
            self.inc_counter();
        }

        if last_offset != buffer_len {
            core_encrypt(&self.block, &mut self.overflow.buffer);
            self.inc_counter();
            self.overflow.offset = 0;
            self.overflow.xor_to(&mut buffer[last_offset..]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quarterround_test_dataset_1() {
        assert_eq!(
            quarterround(0x00000000, 0x00000000, 0x00000000, 0x00000000),
            [0x00000000, 0x00000000, 0x00000000, 0x00000000]
        );
        assert_eq!(
            quarterround(0xe7e8c006, 0xc4f9417d, 0x6479b4b2, 0x68c67137),
            [0xe876d72b, 0x9361dfd5, 0xf1460244, 0x948541a3]
        );
    }

    #[test]
    fn doubleround_test_dataset_1() {
        let input_data = [
            0x00000001, 0x00000000, 0x00000000, 0x00000000,
            0x00000000, 0x00000000, 0x00000000, 0x00000000,
            0x00000000, 0x00000000, 0x00000000, 0x00000000,
            0x00000000, 0x00000000, 0x00000000, 0x00000000
        ];

        let expected_data = [
            0x8186a22d, 0x0040a284, 0x82479210, 0x06929051,
            0x08000090, 0x02402200, 0x00004000, 0x00800000,
            0x00010200, 0x20400000, 0x08008104, 0x00000000,
            0x20500000, 0xa0000040, 0x0008180a, 0x612a8020
        ];

        assert_eq!(doubleround(input_data), expected_data);
    }

    #[test]
    fn doubleround_test_dataset_2() {
        let input_data = [
            0xde501066, 0x6f9eb8f7, 0xe4fbbd9b, 0x454e3f57,
            0xb75540d3, 0x43e93a4c, 0x3a6f2aa0, 0x726d6b36,
            0x9243f484, 0x9145d1e8, 0x4fa9d247, 0xdc8dee11,
            0x054bf545, 0x254dd653, 0xd9421b6d, 0x67b276c1
        ];

        let expected_data = [
            0xccaaf672, 0x23d960f7, 0x9153e63a, 0xcd9a60d0,
            0x50440492, 0xf07cad19, 0xae344aa0, 0xdf4cfdfc,
            0xca531c29, 0x8e7943db, 0xac1680cd, 0xd503ca00,
            0xa74b2ad6, 0xbc331c5c, 0x1dda24c7, 0xee928277
        ];

        assert_eq!(doubleround(input_data), expected_data);
    } 

    #[test]
    fn core_test_dataset_1() {
        let input_data = [0; 16];
        let expected_data = [0; 64];
        let mut hash_data = [0; 64];

        core_generate(&input_data, &mut hash_data);
        assert_eq!(hash_data.to_vec(), expected_data.to_vec());
    }

    #[test]
    fn core_test_dataset_2() {
        let input_data = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5]; 

        let expected_data = [
            127, 190, 243, 215, 29, 98, 52, 193, 21, 188, 119, 254, 16, 167,
            112, 154, 6, 232, 205, 233, 243, 80, 194, 140, 127, 11, 194, 60,
            53, 212, 23, 42, 250, 127, 134, 73, 254, 33, 121, 161, 108, 130,
            64, 60, 55, 197, 79, 88, 23, 155, 192, 203, 232, 169, 108, 94,
            190, 196, 68, 81, 49, 214, 34, 251
        ];

        let mut hash_data = [0; 64];

        core_generate(&input_data, &mut hash_data);
        assert_eq!(hash_data.to_vec(), expected_data.to_vec());
    }

    #[test]
    fn generate_test_key_32() {
        let key: [u8; 32] = [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
            16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1
        ];
        let nonce = [101, 102, 103, 104, 105, 106, 107, 108];
        let mut salsa20 = Salsa20::new(&key, &nonce, 0);

        let mut buffer = [0; 64];
        let expected_data = [
            18, 151, 139, 216, 17, 224, 46, 71, 160, 193, 230, 100, 172, 120,
            246, 93, 95, 171, 234, 5, 244, 163, 188, 198, 240, 72, 180, 58,
            46, 87, 13, 220, 178, 179, 195, 166, 65, 98, 167, 19, 168, 221,
            73, 21, 205, 93, 139, 97, 254, 29, 39, 66, 14, 90, 123, 114, 195,
            159, 46, 6, 177, 250, 152, 39
        ];

        salsa20.generate(&mut buffer);
        assert_eq!(buffer.to_vec(), expected_data.to_vec());
    }

    #[test]
    fn generate_test_key_16() {
        let key: [u8; 16] = [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16
        ];
        let nonce = [101, 102, 103, 104, 105, 106, 107, 108];
        let mut salsa20 = Salsa20::new(&key, &nonce, 0);

        let mut buffer = [0; 64];
        let expected_data = [
            50, 222, 161, 111, 27, 57, 4, 35, 57, 163, 170, 51, 189, 79, 106,
            98, 36, 244, 216, 222, 60, 44, 82, 56, 178, 16, 176, 53, 72, 113,
            210, 220, 125, 79, 174, 182, 250, 151, 108, 127, 226, 128, 36, 88,
            28, 221, 216, 76, 78, 226, 78, 9, 43, 250, 94, 158, 108, 119, 253,
            36, 22, 33, 10, 155
        ];

        salsa20.generate(&mut buffer);
        assert_eq!(buffer.to_vec(), expected_data.to_vec());
    }

    #[test]
    fn generate_test_with_overflow_1() {
        let key: [u8; 32] = [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
            16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1
        ];
        let nonce = [101, 102, 103, 104, 105, 106, 107, 108];
        let mut salsa20 = Salsa20::new(&key, &nonce, 0);

        let mut buffer = [0; 10];
        let expected_data: [[u8; 10]; 6] = [
            [18, 151, 139, 216, 17, 224, 46, 71, 160, 193],
            [230, 100, 172, 120, 246, 93, 95, 171, 234, 5],
            [244, 163, 188, 198, 240, 72, 180, 58, 46, 87],
            [13, 220, 178, 179, 195, 166, 65, 98, 167, 19],
            [168, 221, 73, 21, 205, 93, 139, 97, 254, 29],
            [39, 66, 14, 90, 123, 114, 195, 159, 46, 6]
        ];

        salsa20.generate(&mut buffer);
        assert_eq!(buffer.to_vec(), expected_data[0].to_vec());
        salsa20.generate(&mut buffer);
        assert_eq!(buffer.to_vec(), expected_data[1].to_vec());
        salsa20.generate(&mut buffer);
        assert_eq!(buffer.to_vec(), expected_data[2].to_vec());
        salsa20.generate(&mut buffer);
        assert_eq!(buffer.to_vec(), expected_data[3].to_vec());
        salsa20.generate(&mut buffer);
        assert_eq!(buffer.to_vec(), expected_data[4].to_vec());
        salsa20.generate(&mut buffer);
        assert_eq!(buffer.to_vec(), expected_data[5].to_vec());
    }

    #[test]
    fn generate_test_with_overflow_2() {
        let key: [u8; 32] = [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
            16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1
        ];
        let nonce = [101, 102, 103, 104, 105, 106, 107, 108];

        let mut salsa20_1 = Salsa20::new(&key, &nonce, 0);
        let mut salsa20_2 = Salsa20::new(&key, &nonce, 0);

        let mut buffer = [0; 1024];
        let mut expected_data = [0; 1024];

        salsa20_1.generate(&mut expected_data);

        salsa20_2.generate(&mut buffer[0..100]);
        salsa20_2.generate(&mut buffer[100..253]);
        salsa20_2.generate(&mut buffer[253..578]);
        salsa20_2.generate(&mut buffer[578..934]);
        salsa20_2.generate(&mut buffer[934..1024]);

        assert_eq!(buffer.to_vec(), expected_data.to_vec());
    }
}
