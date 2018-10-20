
pub fn pid0() {
  loop {
    vga_println!("Hello from PID0!");
    let initramfs = include_bytes!("../../initramfs.bin");
    vga_println!("Loading {} byte long initramfs", initramfs.len());
  }
}