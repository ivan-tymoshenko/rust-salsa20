pub(super) fn u8_to_u32(bytes: &[u8], u32_slice: &mut [u32]) {
    for (index, value) in u32_slice.iter_mut().enumerate() {
        let offset = index * 4;
        *value = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3]
        ]);
    }
}

pub(super) fn xor_from_slice(to: &mut [u8], from: &[u8]) {
    for (to_byte, from_byte) in to.iter_mut().zip(from.iter()) {
        *to_byte ^= from_byte;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u8_to_u32_test() {
        test(&[0, 0, 0, 0], &[0]);
        test(&[1, 0, 0, 0], &[1]);
        test(&[1, 2, 3, 4], &[67305985]);
        test(&[1, 2, 3, 4, 5], &[67305985]);

        fn test(bytes: &[u8], expected_values: &[u32]) {
            let mut values = [0];
            u8_to_u32(&bytes, &mut values);
            assert_eq!(values, expected_values);
        }
    }

    #[test]
    fn xor_from_slice_test() {
        test(&mut [0, 0, 0, 0], &[0, 0, 0, 0], &[0, 0, 0, 0]);
        test(&mut [0, 0, 0, 1], &[0, 0, 0, 0], &[0, 0, 0, 1]);
        test(&mut [1, 0, 1, 0], &[1, 1, 0, 0], &[0, 1, 1, 0]);
        test(&mut [1, 2, 3, 4], &[5, 6, 7, 8], &[4, 4, 4, 12]);
        test(&mut [1, 2, 3, 4, 0, 0], &[5, 6, 7, 8], &[4, 4, 4, 12, 0, 0]);

        fn test(to: &mut [u8], from: &[u8], expected: &[u8]) {
            xor_from_slice(to, from);
            assert_eq!(to, expected);
        }
    }
}
