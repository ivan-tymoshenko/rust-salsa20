#![no_std]

mod utils;
use crate::utils::{u8_to_u32, xor_from_slice};

fn quarterround(y0: u32, y1: u32, y2: u32, y3: u32) -> [u32; 4] {
    let y1 = y1 ^ y0.wrapping_add(y3).rotate_left(7);
    let y2 = y2 ^ y1.wrapping_add(y0).rotate_left(9);
    let y3 = y3 ^ y2.wrapping_add(y1).rotate_left(13);
    let y0 = y0 ^ y3.wrapping_add(y2).rotate_left(18);

    [y0, y1, y2, y3]
}

fn columnround(y: [u32; 16]) -> [u32; 16] {
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

    [z0, z1, z2, z3, z4, z5, z6, z7, z8, z9, z10, z11, z12, z13, z14, z15]
}

fn rowround(y: [u32; 16]) -> [u32; 16] {
    let [
        [z0, z1, z2, z3],
        [z5, z6, z7, z4],
        [z10, z11, z8, z9],
        [z15, z12, z13, z14]
    ] = [
        quarterround(y[0], y[1], y[2], y[3]),
        quarterround(y[5], y[6], y[7], y[4]),
        quarterround(y[10], y[11], y[8], y[9]),
        quarterround(y[15], y[12], y[13], y[14])
    ];

    [z0, z1, z2, z3, z4, z5, z6, z7, z8, z9, z10, z11, z12, z13, z14, z15]
}

#[inline(always)]
fn doubleround(y: [u32; 16]) -> [u32; 16] {
    rowround(columnround(y))
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

    fn modify<F>(&mut self, buffer: &mut [u8], modifier: F)
        where F: Fn(&mut [u8], &[u8])
    {
        let offset = self.offset;
        self.offset += buffer.len();
        modifier(buffer, &self.buffer[offset..self.offset]);
    }
}

#[derive(Clone, Copy)]
struct Generator {
    init_matrix: [u32; 16],
    cround_matrix: [u32; 16],
    dround_values: [u32; 4],
    counter: u64
}

impl Generator {
    fn new(key: &[u8], nonce: &[u8; 8], counter: u64) -> Generator {
        let mut init_matrix = [0; 16];
        init_matrix[0] = 1634760805;
        init_matrix[15] = 1797285236;
        init_matrix[8] = counter as u32;
        init_matrix[9] = (counter >> 32) as u32;
        u8_to_u32(&nonce[..], &mut init_matrix[6..8]);

        match key.len() {
            16 => {
                u8_to_u32(&key[..], &mut init_matrix[1..5]);
                u8_to_u32(&key[..], &mut init_matrix[11..15]);
                init_matrix[5] = 824206446;
                init_matrix[10] = 2036477238;
            }
            32 => {
                u8_to_u32(&key[..16], &mut init_matrix[1..5]);
                u8_to_u32(&key[16..], &mut init_matrix[11..15]);
                init_matrix[5] = 857760878;
                init_matrix[10] = 2036477234;
            } _ => {
                panic!("Wrong key size.");
            }
        }
        let cround_matrix = columnround(init_matrix);
        let dround_values = quarterround(
            cround_matrix[5],
            cround_matrix[6],
            cround_matrix[7],
            cround_matrix[4]
        );

        Generator { init_matrix, cround_matrix, dround_values, counter }
    }

    fn first_doubleround(&self) -> [u32; 16] {
        let [r5, r6, r7, r4] = self.dround_values;
        let [
            [r0, r1, r2, r3],
            [r10, r11, r8, r9],
            [r15, r12, r13, r14]
        ] = [
            quarterround(
                self.cround_matrix[0],
                self.cround_matrix[1],
                self.cround_matrix[2],
                self.cround_matrix[3]
            ),
            quarterround(
                self.cround_matrix[10],
                self.cround_matrix[11],
                self.cround_matrix[8],
                self.cround_matrix[9]
            ),
            quarterround(
                self.cround_matrix[15],
                self.cround_matrix[12],
                self.cround_matrix[13],
                self.cround_matrix[14]
            )
        ];

        [r0, r1, r2, r3, r4, r5, r6, r7, r8, r9, r10, r11, r12, r13, r14, r15]
    }

    fn set_counter(&mut self, counter: u64) {
        self.counter = counter;
        self.init_matrix[8] = counter as u32;
        let [z0, z4, z8, z12] = quarterround(
            self.init_matrix[0],
            self.init_matrix[4],
            self.init_matrix[8],
            self.init_matrix[12]
        );
        self.cround_matrix[0] = z0;
        self.cround_matrix[8] = z8;
        self.cround_matrix[12] = z12;

        if counter > 0xffffffff_u64 {
            self.init_matrix[9] = (counter >> 32) as u32;
            let [z5, z9, z13, z1] = quarterround(
                self.init_matrix[5],
                self.init_matrix[9],
                self.init_matrix[13],
                self.init_matrix[1]
            );

            self.cround_matrix[1] = z1;
            self.cround_matrix[9] = z9;
            self.cround_matrix[13] = z13;

            self.dround_values = quarterround(
                z5,
                self.cround_matrix[6],
                self.cround_matrix[7],
                z4
            );
        }
    }

    fn generate<F>(&mut self, buffer: &mut[u8], modifier: F)
        where F: Fn(&mut [u8], &[u8])
    {
        (0..9)
            .fold(self.first_doubleround(), |block, _| doubleround(block))
            .iter()
            .zip(self.init_matrix.iter())
            .enumerate()
            .for_each(|(index, (drounds_value, &init_value))| {
                let offset = index * 4;
                let sum = drounds_value.wrapping_add(init_value);
                modifier(&mut buffer[offset..offset + 4], &sum.to_le_bytes());
            });

        self.set_counter(self.counter.wrapping_add(1));
    }
}

#[derive(Clone, Copy)]
pub struct Salsa20 {
    generator: Generator,
    overflow: Overflow
}

impl Salsa20 {
    pub fn new(key: &[u8], nonce: &[u8; 8], counter: u64) -> Salsa20 {
        let overflow = Overflow::new();
        let generator = Generator::new(key, nonce, counter);
        Salsa20 { generator, overflow }
    }

    fn modify<F>(&mut self, buffer: &mut [u8], modifier: &F)
        where F: Fn(&mut [u8], &[u8])
    {
        let buffer_len = buffer.len();
        let overflow_len = 64 - self.overflow.offset;

        if overflow_len != 0 {
            if buffer_len >= overflow_len {
                self.overflow.modify(&mut buffer[..overflow_len], modifier);
            } else {
                self.overflow.modify(&mut buffer[..], modifier);
                return;
            }
        }

        let last_block_offset = buffer_len - (buffer_len - overflow_len) % 64;

        for offset in (overflow_len..last_block_offset).step_by(64) {
            self.generator.generate(&mut buffer[offset..offset + 64], modifier);
        }

        if last_block_offset != buffer_len {
            self.generator.generate(
                &mut self.overflow.buffer,
                <[u8]>::copy_from_slice
            );
            self.overflow.offset = 0;
            self.overflow.modify(&mut buffer[last_block_offset..], modifier);
        }
    }

    pub fn set_counter(&mut self, counter: u64) {
        self.generator.set_counter(counter);
    }

    pub fn generate(&mut self, buffer: &mut [u8]) {
        self.modify(buffer, &<[u8]>::copy_from_slice);
    }

    pub fn encrypt(&mut self, buffer: &mut [u8]) {
        self.modify(buffer, &xor_from_slice);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quarterround_test() {
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
    fn rowround_test() {
        test([
            0x00000001, 0x00000000, 0x00000000, 0x00000000,
            0x00000001, 0x00000000, 0x00000000, 0x00000000,
            0x00000001, 0x00000000, 0x00000000, 0x00000000,
            0x00000001, 0x00000000, 0x00000000, 0x00000000
        ], [
            0x08008145, 0x00000080, 0x00010200, 0x20500000,
            0x20100001, 0x00048044, 0x00000080, 0x00010000,
            0x00000001, 0x00002000, 0x80040000, 0x00000000,
            0x00000001, 0x00000200, 0x00402000, 0x88000100
        ]);

        test([
             0x08521bd6, 0x1fe88837, 0xbb2aa576, 0x3aa26365,
             0xc54c6a5b, 0x2fc74c2f, 0x6dd39cc3, 0xda0a64f6,
             0x90a2f23d, 0x067f95a6, 0x06b35f61, 0x41e4732e,
             0xe859c100, 0xea4d84b7, 0x0f619bff, 0xbc6e965a
        ], [
            0xa890d39d, 0x65d71596, 0xe9487daa, 0xc8ca6a86,
            0x949d2192, 0x764b7754, 0xe408d9b9, 0x7a41b4d1,
            0x3402e183, 0x3c3af432, 0x50669f96, 0xd89ef0a8,
            0x0040ede5, 0xb545fbce, 0xd257ed4f, 0x1818882d
        ]);

        fn test(input_data: [u32; 16], expected_data: [u32; 16]) {
            assert_eq!(rowround(input_data), expected_data);
        }
    }

    #[test]
    fn columnround_test() {
        test([
            0x00000001, 0x00000000, 0x00000000, 0x00000000,
            0x00000001, 0x00000000, 0x00000000, 0x00000000,
            0x00000001, 0x00000000, 0x00000000, 0x00000000,
            0x00000001, 0x00000000, 0x00000000, 0x00000000
        ], [
            0x10090288, 0x00000000, 0x00000000, 0x00000000,
            0x00000101, 0x00000000, 0x00000000, 0x00000000,
            0x00020401, 0x00000000, 0x00000000, 0x00000000,
            0x40a04001, 0x00000000, 0x00000000, 0x00000000
        ]);

        test([
            0x08521bd6, 0x1fe88837, 0xbb2aa576, 0x3aa26365,
            0xc54c6a5b, 0x2fc74c2f, 0x6dd39cc3, 0xda0a64f6,
            0x90a2f23d, 0x067f95a6, 0x06b35f61, 0x41e4732e,
            0xe859c100, 0xea4d84b7, 0x0f619bff, 0xbc6e965a
        ], [
            0x8c9d190a, 0xce8e4c90, 0x1ef8e9d3, 0x1326a71a,
            0x90a20123, 0xead3c4f3, 0x63a091a0, 0xf0708d69,
            0x789b010c, 0xd195a681, 0xeb7d5504, 0xa774135c,
            0x481c2027, 0x53a8e4b5, 0x4c1f89c5, 0x3f78c9c8
        ]);

        fn test(input_data: [u32; 16], expected_data: [u32; 16]) {
            assert_eq!(columnround(input_data), expected_data);
        }
    }

    #[test]
    fn doubleround_test() {
        test([
            0x00000001, 0x00000000, 0x00000000, 0x00000000,
            0x00000000, 0x00000000, 0x00000000, 0x00000000,
            0x00000000, 0x00000000, 0x00000000, 0x00000000,
            0x00000000, 0x00000000, 0x00000000, 0x00000000
        ], [
            0x8186a22d, 0x0040a284, 0x82479210, 0x06929051,
            0x08000090, 0x02402200, 0x00004000, 0x00800000,
            0x00010200, 0x20400000, 0x08008104, 0x00000000,
            0x20500000, 0xa0000040, 0x0008180a, 0x612a8020
        ]);

        test([
            0xde501066, 0x6f9eb8f7, 0xe4fbbd9b, 0x454e3f57,
            0xb75540d3, 0x43e93a4c, 0x3a6f2aa0, 0x726d6b36,
            0x9243f484, 0x9145d1e8, 0x4fa9d247, 0xdc8dee11,
            0x054bf545, 0x254dd653, 0xd9421b6d, 0x67b276c1
        ], [
            0xccaaf672, 0x23d960f7, 0x9153e63a, 0xcd9a60d0,
            0x50440492, 0xf07cad19, 0xae344aa0, 0xdf4cfdfc,
            0xca531c29, 0x8e7943db, 0xac1680cd, 0xd503ca00,
            0xa74b2ad6, 0xbc331c5c, 0x1dda24c7, 0xee928277
        ]);

        fn test(input_data: [u32; 16], expected_data: [u32; 16]) {
            assert_eq!(doubleround(input_data), expected_data);
        }
    }

    #[test]
    fn create_init_matrix_test() {
        test(&[
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16
        ], [
            101, 120, 112, 97, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14,
            15, 16, 110, 100, 32, 49, 101, 102, 103, 104, 105, 106, 107, 108,
            109, 110, 111, 112, 113, 114, 115, 116, 54, 45, 98, 121, 1, 2, 3,
            4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 116, 101, 32, 107
        ]);

        test(&[
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 201, 202,
            203, 204, 205, 206, 207, 208, 209, 210, 211, 212, 213, 214, 215, 216
        ], [
            101, 120, 112, 97, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14,
            15, 16, 110, 100, 32, 51, 101, 102, 103, 104, 105, 106, 107, 108,
            109, 110, 111, 112, 113, 114, 115, 116, 50, 45, 98, 121, 201, 202,
            203, 204, 205, 206, 207, 208, 209, 210, 211, 212, 213, 214, 215,
            216, 116, 101, 32, 107
        ]);

        fn test(key: &[u8], expected_data: [u8; 64]) {
            let nonce = [101, 102, 103, 104, 105, 106, 107, 108];
            let counter = u64::from_le_bytes(
                [109, 110, 111, 112, 113, 114, 115, 116]
            );
            let generator = Generator::new(&key, &nonce, counter);

            let mut expected_data_u32 = [0; 16];
            u8_to_u32(&expected_data, &mut expected_data_u32);
            assert_eq!(generator.init_matrix, expected_data_u32);
        }
    }

    #[test]
    fn first_doubleround_test() {
        test(0x00000000, [0x00000000, 0x00000000]);
        test(0x00000001, [0x00000001, 0x00000000]);
        test(0x1234567f, [0x1234567f, 0x00000000]);
        test(0xffffffff, [0xffffffff, 0x00000000]);
        test(0x100000000, [0x00000000, 0x00000001]);
        test(0x012345678abcdef, [0x78abcdef, 0x123456]);

        fn test(counter: u64, counter_as_u32: [u32; 2]) {
            let mut generator = Generator::new(&[0; 16], &[0; 8], 0);
            generator.set_counter(counter);
            assert_eq!(generator.init_matrix[8..10], counter_as_u32);
            assert_eq!(
                generator.first_doubleround(),
                doubleround(generator.init_matrix)
            );
        };
    }

    #[test]
    fn generate_test() {
        test(&[
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16
        ], [
            39, 173, 46, 248, 30, 200, 82, 17, 48, 67, 254, 239, 37, 18, 13,
            247, 241, 200, 61, 144, 10, 55, 50, 185, 6, 47, 246, 253, 143, 86,
            187, 225, 134, 85, 110, 246, 161, 163, 43, 235, 231, 94, 171, 51,
            145, 214, 112, 29, 14, 232, 5, 16, 151, 140, 183, 141, 171, 9, 122,
            181, 104, 182, 177, 193
        ]);

        test(&[
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 201, 202,
            203, 204, 205, 206, 207, 208, 209, 210, 211, 212, 213, 214, 215, 216
        ], [
            69, 37, 68, 39, 41, 15, 107, 193, 255, 139, 122, 6, 170, 233, 217,
            98, 89, 144, 182, 106, 21, 51, 200, 65, 239, 49, 222, 34, 215, 114,
            40, 126, 104, 197, 7, 225, 197, 153, 31, 2, 102, 78, 76, 176, 84,
            245, 246, 184, 177, 160, 133, 130, 6, 72, 149, 119, 192, 195, 132,
            236, 234, 103, 246, 74
        ]);

        fn test(key: &[u8], expected_data: [u8; 64]) {
            let nonce = [101, 102, 103, 104, 105, 106, 107, 108];
            let counter = u64::from_le_bytes(
                [109, 110, 111, 112, 113, 114, 115, 116]
            );
            let mut generator = Generator::new(&key, &nonce, counter);

            let mut buffer = [0; 64];
            generator.generate(&mut buffer, <[u8]>::copy_from_slice);
            assert_eq!(buffer.to_vec(), expected_data.to_vec());
        }
    }

    #[test]
    fn generate_with_overflow_test() {
        test(&[
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16
        ], [
            39, 173, 46, 248, 30, 200, 82, 17, 48, 67, 254, 239, 37, 18, 13,
            247, 241, 200, 61, 144, 10, 55, 50, 185, 6, 47, 246, 253, 143, 86,
            187, 225, 134, 85, 110, 246, 161, 163, 43, 235, 231, 94, 171, 51,
            145, 214, 112, 29, 14, 232, 5, 16, 151, 140, 183, 141, 171, 9, 122,
            181, 104, 182, 177, 193
        ]);

        test(&[
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 201, 202,
            203, 204, 205, 206, 207, 208, 209, 210, 211, 212, 213, 214, 215, 216
        ], [
            69, 37, 68, 39, 41, 15, 107, 193, 255, 139, 122, 6, 170, 233, 217,
            98, 89, 144, 182, 106, 21, 51, 200, 65, 239, 49, 222, 34, 215, 114,
            40, 126, 104, 197, 7, 225, 197, 153, 31, 2, 102, 78, 76, 176, 84,
            245, 246, 184, 177, 160, 133, 130, 6, 72, 149, 119, 192, 195, 132,
            236, 234, 103, 246, 74
        ]);

        fn test(key: &[u8], expected_data: [u8; 64]) {
            let nonce = [101, 102, 103, 104, 105, 106, 107, 108];
            let counter = u64::from_le_bytes(
                [109, 110, 111, 112, 113, 114, 115, 116]
            );
            let mut salsa = Salsa20::new(&key, &nonce, counter);

            let mut buffer = [0; 64];
            salsa.generate(&mut buffer[..8]);
            salsa.generate(&mut buffer[8..17]);
            salsa.generate(&mut buffer[17..29]);
            salsa.generate(&mut buffer[29..64]);
            assert_eq!(buffer.to_vec(), expected_data.to_vec());
        }
    }

    #[test]
    fn encrypt_test() {
        test(&[
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16
        ], [
            38, 172, 47, 249, 31, 201, 83, 16, 49, 66, 255, 238, 36, 19, 12,
            246, 240, 201, 60, 145, 11, 54, 51, 184, 7, 46, 247, 252, 142, 87,
            186, 224, 135, 84, 111, 247, 160, 162, 42, 234, 230, 95, 170, 50,
            144, 215, 113, 28, 15, 233, 4, 17, 150, 141, 182, 140, 170, 8, 123,
            180, 105, 183, 176, 192
        ]);

        test(&[
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 201, 202,
            203, 204, 205, 206, 207, 208, 209, 210, 211, 212, 213, 214, 215, 216
        ], [
            68, 36, 69, 38, 40, 14, 106, 192, 254, 138, 123, 7, 171, 232, 216,
            99, 88, 145, 183, 107, 20, 50, 201, 64, 238, 48, 223, 35, 214, 115,
            41, 127, 105, 196, 6, 224, 196, 152, 30, 3, 103, 79, 77, 177, 85,
            244, 247, 185, 176, 161, 132, 131, 7, 73, 148, 118, 193, 194, 133,
            237, 235, 102, 247, 75
        ]);

        fn test(key: &[u8], expected_data: [u8; 64]) {
            let nonce = [101, 102, 103, 104, 105, 106, 107, 108];
            let counter = u64::from_le_bytes(
                [109, 110, 111, 112, 113, 114, 115, 116]
            );
            let mut generator = Generator::new(&key, &nonce, counter);

            let mut buffer = [1; 64];
            generator.generate(&mut buffer, xor_from_slice);
            assert_eq!(buffer.to_vec(), expected_data.to_vec());
        }
    }

    #[test]
    fn encrypt_with_overflow_test() {
        test(&[
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16
        ], [
            38, 172, 47, 249, 31, 201, 83, 16, 49, 66, 255, 238, 36, 19, 12,
            246, 240, 201, 60, 145, 11, 54, 51, 184, 7, 46, 247, 252, 142, 87,
            186, 224, 135, 84, 111, 247, 160, 162, 42, 234, 230, 95, 170, 50,
            144, 215, 113, 28, 15, 233, 4, 17, 150, 141, 182, 140, 170, 8, 123,
            180, 105, 183, 176, 192
        ]);

        test(&[
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 201, 202,
            203, 204, 205, 206, 207, 208, 209, 210, 211, 212, 213, 214, 215, 216
        ], [
            68, 36, 69, 38, 40, 14, 106, 192, 254, 138, 123, 7, 171, 232, 216,
            99, 88, 145, 183, 107, 20, 50, 201, 64, 238, 48, 223, 35, 214, 115,
            41, 127, 105, 196, 6, 224, 196, 152, 30, 3, 103, 79, 77, 177, 85,
            244, 247, 185, 176, 161, 132, 131, 7, 73, 148, 118, 193, 194, 133,
            237, 235, 102, 247, 75
        ]);

        fn test(key: &[u8], expected_data: [u8; 64]) {
            let nonce = [101, 102, 103, 104, 105, 106, 107, 108];
            let counter = u64::from_le_bytes(
                [109, 110, 111, 112, 113, 114, 115, 116]
            );
            let mut salsa = Salsa20::new(&key, &nonce, counter);

            let mut buffer = [1; 64];
            salsa.encrypt(&mut buffer[..8]);
            salsa.encrypt(&mut buffer[8..17]);
            salsa.encrypt(&mut buffer[17..29]);
            salsa.encrypt(&mut buffer[29..64]);
            assert_eq!(buffer.to_vec(), expected_data.to_vec());
        }
    }
}
