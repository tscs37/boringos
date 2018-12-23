#![no_std]
#![feature(start)]
#![feature(alloc_error_handler)] 

#[start]
fn main(_: isize, syscp: *const *const u8) -> isize {
    let mut sp = uart_16550::SerialPort::new(0x3F8);
    sp.init();
    use core::fmt::Write;
    sp.write_str("Hello, world!").unwrap();
    sp.write_fmt(format_args!("SYSCP={:#018x}", syscp as u64));
    loop{}
}

use core::panic::PanicInfo;

/// This function is called on panic.
#[panic_handler]
#[no_mangle]
pub fn panic(info: &PanicInfo) -> ! {
  loop{}
}

#[alloc_error_handler]
#[no_mangle]
pub fn alloc_error(layout: core::alloc::Layout) -> ! {
  loop{}
}