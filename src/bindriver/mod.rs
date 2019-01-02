
#[macro_use]
pub mod vga_buffer;
pub mod serial;
pub mod qemu;
pub mod cpu;

pub fn init() {
  crate::bindriver::serial::init();
  crate::bindriver::cpu::enable_nxe_bit();
  crate::bindriver::cpu::gdt::init();
  crate::bindriver::cpu::idt::init();
  crate::bindriver::cpu::pic::init();
}