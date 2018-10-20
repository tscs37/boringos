
#[naked]
pub fn pid0() {
  debug!("PID 0 running");
  dump_stack_addr!();
  let initramfs = include_bytes!("../../initramfs.bin");
  debug!("Loading {} byte long initramfs", initramfs.len());
  debug!("Starting RNG Provider...");
  //TODO:
  debug!("Registering Handle Numbering Provider...");
  //TODO:
  debug!("Starting Time Provider...");
  //TODO:
  debug!("Starting PVFS...");
  //TODO:
  debug!("Starting PVFS:Tar...");
  //TODO:
  debug!("Starting Scheduler...");
  //TODO:
  debug!("Marking self as shelled out...");
  //TODO:
  loop{}
}