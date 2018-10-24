
#[naked]
pub fn pid0() {
  debug!("PID 0 running");
  dump_stack_addr!();
  let initramfs = include_bytes!("../../initramfs.bin");
  debug!("Loading {} byte long initramfs", initramfs.len());
  debug!("Creating Scheduler Task... ");
  let ph = ipc_call!(bos_new_process);
  //TODO:
  proc_yield!(); // Yield once to make sure everything is alright
  debug!("Starting Time Provider...");
  //TODO:
  debug!("Starting PVFS...");
  //TODO:
  debug!("Starting PVFS:Tar...");
  // work in progress (??)
  ipc_call!(pvfs_register_fs_driver, "pvfs_tar");
  //TODO:
  debug!("Starting Scheduler...");
  //TODO:
  debug!("Marking self as shelled out...");
  //TODO:
  loop{}
}