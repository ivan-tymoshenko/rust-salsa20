#![no_std]

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

fn xor_from_slice(from: &[u8], to: &mut [u8]) {
    for (to_byte, from_byte) in to.iter_mut().zip(from.iter()) {
        *to_byte ^= *from_byte;
    }
}

fn copy_from_slice(from: &[u8], to: &mut [u8]) {
    to.copy_from_slice(from);
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
        where F: Fn(&[u8], &mut [u8])
    {
        let offset = self.offset;
        self.offset += buffer.len();
        modifier(&self.buffer[offset..self.offset], buffer);
    }
}

#[derive(Clone, Copy)]
struct Generator {
    y: [u32; 16],
    z: [u32; 16],
    r: [u32; 4],
    counter: u64
}

impl Generator {
    fn new(key: &[u8], nonce: &[u8; 8], counter: u64) -> Generator {
        let mut y = [0; 16];
        y[0] = 1634760805;
        y[15] = 1797285236;
        y[8] = counter as u32;
        y[9] = (counter >> 32) as u32;
        u8_to_u32(&nonce[..], &mut y[6..8]);

        match key.len() {
            16 => {
                u8_to_u32(&key[..], &mut y[1..5]);
                u8_to_u32(&key[..], &mut y[11..15]);
                y[5] = 824206446;
                y[10] = 1885482294;
            }
            32 => {
                u8_to_u32(&key[..16], &mut y[1..5]);
                u8_to_u32(&key[16..], &mut y[11..15]);
                y[5] = 824206446;
                y[10] = 2036477238;
            } _ => {
                panic!("Wrong key size.");
            }
        }
        let z = columnround(y);
        let r = quarterround(z[5], z[6], z[7], z[4]);

        Generator { y, z, r, counter }
    }

    fn first_doubleround(&self) -> [u32; 16] {
        let [r5, r6, r7, r4] = self.r;
        let [
            [r0, r1, r2, r3],
            [r10, r11, r8, r9],
            [r15, r12, r13, r14]
        ] = [
            quarterround(self.z[0], self.z[1], self.z[2], self.z[3]),
            quarterround(self.z[10], self.z[11], self.z[8], self.z[9]),
            quarterround(self.z[15], self.z[12], self.z[13], self.z[14])
        ];
 
        [r0, r1, r2, r3, r4, r5, r6, r7, r8, r9, r10, r11, r12, r13, r14, r15]
    }

    fn set_counter(&mut self, counter: u64) {
        self.counter = counter;
        self.y[8] = counter as u32;
        let [z0, z4, z8, z12] = quarterround(
            self.y[0],
            self.y[4],
            self.y[8],
            self.y[12]
        );
        self.z[0] = z0;
        self.z[8] = z8;
        self.z[12] = z12;

        if counter > 0xffffffff_u64 {
            self.y[9] = (counter >> 32) as u32;
            let [z5, z9, z13, z1] = quarterround(
                self.y[1],
                self.y[5],
                self.y[9],
                self.y[13]
            );
            self.z[1] = z1;
            self.z[9] = z9;
            self.z[13] = z13;

            self.r = quarterround(z5, self.z[6], self.z[7], z4);
        }
    }

    fn generate<F>(&mut self, result: &mut[u8], modifier: F)
        where F: Fn(&[u8], &mut [u8])
    {
        (0..9)
            .fold(self.first_doubleround(), |block, _| doubleround(block))
            .iter()
            .zip(self.y.iter())
            .enumerate()
            .for_each(|(index, (value, &value_copy))| {
                let offset = index * 4;
                let sum = value.wrapping_add(value_copy); 
                modifier(&sum.to_le_bytes(), &mut result[offset..offset + 4]);
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
        where F: Fn(&[u8], &mut [u8])
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

        let last_offset = buffer_len - (buffer_len - overflow_len) % 64; 

        for offset in (overflow_len..last_offset).step_by(64) {
            self.generator.generate(&mut buffer[offset..offset + 64], modifier);
        }

        if last_offset != buffer_len {
            self.generator.generate(&mut self.overflow.buffer, copy_from_slice);
            self.overflow.offset = 0;
            self.overflow.modify(&mut buffer[last_offset..], modifier);
        }
    }

    pub fn set_counter(&mut self, counter: u64) {
        self.generator.set_counter(counter);
    }

    pub fn generate(&mut self, buffer: &mut [u8]) {
        self.modify(buffer, &copy_from_slice);
    }
    
    pub fn encrypt(&mut self, buffer: &mut [u8]) {
        self.modify(buffer, &xor_from_slice);
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

    fn create_salsa20(key_size: u8) -> Salsa20 {
        let key_16 = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let key_32 = [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
            16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1
        ];

        let nonce = [101, 102, 103, 104, 105, 106, 107, 108];

        match key_size {
            16 => Salsa20::new(&key_16, &nonce, 0),
            _ => Salsa20::new(&key_32, &nonce, 0)
        }
    }

    #[test]
    fn generate_test_key_32() {
        let mut salsa20 = create_salsa20(32);

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

    #[test]
    fn encrypt_test_dataset_1() {
        let key: [u8; 16] = [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16
        ];
        let nonce = [101, 102, 103, 104, 105, 106, 107, 108];
        let mut salsa20 = Salsa20::new(&key, &nonce, 0);

        let mut buffer = [7; 64];
        let expected_data = [
            53, 217, 166, 104, 28, 62, 3, 36, 62, 164, 173, 52, 186, 72, 109,
            101, 35, 243, 223, 217, 59, 43, 85, 63, 181, 23, 183, 50, 79, 118,
            213, 219, 122, 72, 169, 177, 253, 144, 107, 120, 229, 135, 35, 95,
            27, 218, 223, 75, 73, 229, 73, 14, 44, 253, 89, 153, 107, 112, 250,
            35, 17, 38, 13, 156
        ];

        salsa20.encrypt(&mut buffer);
        assert_eq!(buffer.to_vec(), expected_data.to_vec());
    }
}
