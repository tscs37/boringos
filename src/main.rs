
#![feature(panic_handler)]
#![feature(abi_x86_interrupt)]
#![no_std]
#![no_main]

#[macro_use]
extern crate lazy_static;
extern crate bootloader;
extern crate volatile;
extern crate spin;
extern crate uart_16550;
extern crate x86_64;
#[macro_use]
mod bindriver;
mod version;

#[no_mangle]
pub extern "C" fn _start() -> ! {
  print_bootup();
  loop {}
}

fn print_bootup() {
  println!("BoringOS v{}\n", version::VERSION);
  print!("Initializing IDT...");
  bindriver::cpu::idt::init_idt();
  print_green!("[ OK ]\n");
  print!("Loading VMEM Driver...");
  //TODO
  print_green!("[ OK ]\n");
  print!("Initializing Process Manager & Environment...");
  print_green!("[ OK ]\n");
  print!("Loading InitRamFS...");
  //TODO
  print_green!("[ OK ]\n");
  print!("Running /bin/init...");

  panic!("Kernel terminated unexpectedly");
}

use core::panic::PanicInfo;

/// This function is called on panic.
#[panic_handler]
#[no_mangle]
pub fn panic(info: &PanicInfo) -> ! {
  use bindriver::vga_buffer::WRITER;
  use bindriver::vga_buffer::helper::{Color,ColorCode};
  WRITER.lock().color_code = ColorCode::new(Color::Red, Color::Black);
  vga_println!("\n\n===== PANIC OCCURED IN KERNEL =====");
  vga_println!("{}", info);

  loop {}
}
