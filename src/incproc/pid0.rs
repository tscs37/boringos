
pub fn pid0() {
  loop {
    println!("Hello from PID0!");
    let initramfs = include_bytes!("../../initramfs.bin");
    println!("Loading {} byte long initramfs", initramfs.len());
  }
}