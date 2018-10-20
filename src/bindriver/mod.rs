
#[macro_use]
pub mod vga_buffer;
pub mod serial;
pub mod qemu;
pub mod cpu;

pub fn init() {
  ::bindriver::serial::init();
  debug!("setting up CPU IDT");
  ::bindriver::cpu::idt::init_idt();
}