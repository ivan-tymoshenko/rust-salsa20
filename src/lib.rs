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

fn hash(data: &[u32; 16], hash: &mut[u8]) {
    let mut data_copy = data.clone();

    for _ in 0..10 {
        doubleround(&mut data_copy);
    }

    data.iter()
        .zip(data_copy.iter())
        .enumerate()
        .for_each(|(index, (value, &value_copy))| {
            let offset = index * 4;
            let sum = value.wrapping_add(value_copy); 
            hash[offset..offset + 4].copy_from_slice(&sum.to_le_bytes());
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

pub struct Salsa20 {
    counter: u64,
    block: [u32; 16],
}

impl Salsa20 {
    pub fn new(key: &[u8], nonce: &[u8; 8], counter: u64) -> Salsa20 {
        let mut block = [0; 16];
        block[0] = 1634760805;
        block[15] = 1797285236;
        u8_to_u32(&nonce[..], &mut block[6..8]);

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

        Salsa20 { block, counter }
    }

    pub fn generate(&mut self, buffer: &mut [u8]) {
        assert_eq!(buffer.len() % 64, 0);

        for offset in (0..buffer.len()).step_by(64) {
           u8_to_u32(&self.counter.to_le_bytes(), &mut self.block[8..10]);
           hash(&self.block, &mut buffer[offset..offset + 64]);
           self.counter += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{doubleround, u8_to_u32, hash, Salsa20};

    #[test]
    fn doubleround_test_dataset_1() {
        let mut input_data = [
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

        doubleround(&mut input_data);
        assert_eq!(input_data, expected_data);
    }

    #[test]
    fn doubleround_test_dataset_2() {
        let mut input_data = [
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

        doubleround(&mut input_data);
        assert_eq!(input_data, expected_data);
    }
    
    #[test]
    fn hash_test_dataset_1() {
        let input_data = [0; 16];
        let expected_data = [0; 64];
        let mut hash_data = [0; 64];

        hash(&input_data, &mut hash_data);
        assert_eq!(hash_data.to_vec(), expected_data.to_vec());
    }

    #[test]
    fn hash_test_dataset_2() {
        let input_data = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5]; 

        let expected_data = [
            127, 190, 243, 215, 29, 98, 52, 193, 21, 188, 119, 254, 16, 167,
            112, 154, 6, 232, 205, 233, 243, 80, 194, 140, 127, 11, 194, 60,
            53, 212, 23, 42, 250, 127, 134, 73, 254, 33, 121, 161, 108, 130,
            64, 60, 55, 197, 79, 88, 23, 155, 192, 203, 232, 169, 108, 94,
            190, 196, 68, 81, 49, 214, 34, 251
        ];

        let mut hash_data = [0; 64];

        hash(&input_data, &mut hash_data);
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
}
