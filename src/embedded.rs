use super::{Key, Salsa20};
use core::slice::from_raw_parts_mut;

#[no_mangle]
pub extern "C" fn salsa20_new(key: &Key, nonce: &[u8; 8], counter: u64) -> Salsa20 {
    Salsa20::new(key, nonce, counter)
}

#[no_mangle]
pub unsafe extern "C" fn encrypt(salsa20: &mut Salsa20, ptr: *mut u8, len: usize) {
    Salsa20::encrypt(salsa20, from_raw_parts_mut(ptr, len));
}

#[no_mangle]
pub unsafe extern "C" fn generate(salsa20: &mut Salsa20, ptr: *mut u8, len: usize) {
    Salsa20::generate(salsa20, from_raw_parts_mut(ptr, len));
}

#[lang = "eh_personality"]
fn eh_personality() {}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
