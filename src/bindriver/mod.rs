
#[macro_use]
pub mod vga_buffer;
pub mod serial;
pub mod qemu;
pub mod cpu;

pub fn init() {
  ::bindriver::serial::init();
  debug!("setting up CPU IDT");
  ::bindriver::cpu::gdt::init();
  ::bindriver::cpu::idt::init();
  ::bindriver::cpu::pic::init();
}