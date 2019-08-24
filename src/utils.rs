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

pub(super) fn xor_from_slice(to: &mut[u8], from: &[u8]) {
    for (to_byte, from_byte) in to.iter_mut().zip(from.iter()) {
        *to_byte ^= from_byte;
    }
}
