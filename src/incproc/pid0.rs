
#[naked]
pub fn pid0() {
  debug!("PID 0 running");
  dump_stack_addr!();
  vga_println!("Hello from PID0!");
  let initramfs = include_bytes!("../../initramfs.bin");
  vga_println!("Loading {} byte long initramfs", initramfs.len());
  loop{}
}