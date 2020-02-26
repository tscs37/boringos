#![feature(abi_x86_interrupt,alloc_error_handler,allocator_api,lang_items,
  const_raw_ptr_to_usize_cast,asm,naked_functions,integer_atomics,panic_info_message,
  const_fn,exclusive_range_pattern,try_trait,concat_idents,custom_test_frameworks)]

#![test_runner(crate::test::test_runner)]
#![reexport_test_harness_main = "test_main"]

#![allow(unused_variables,dead_code)]
#![warn(unused_import_braces)]
#![deny(unused_qualifications,keyword_idents,unused_extern_crates,stable_features)]

#![no_std]
#![no_main]

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate static_assertions;
#[macro_use]
extern crate log;
#[macro_use]
extern crate alloc;

#[macro_use]
mod common;
#[macro_use]
mod bindriver;
mod inc;
mod process_environment;
mod process_manager;
mod version;
mod vmem;
#[cfg(test)]
mod test;

use self::alloc::sync::Arc;
use self::process_manager::Userspace;
use self::vmem::PageManager;
use core::cell::RefCell;

pub use crate::common::*;

pub static PAGER: PageManager = PageManager::new();
static mut USERSPACE: Option<Arc<RefCell<Userspace>>> = None;

bootloader::entry_point!(kernel_main);

fn kernel_main(boot_info: &'static bootloader::BootInfo) -> ! {
  // init drivers for core hardware
  bindriver::init();
  info!("BoringOS v{}", version::VERSION);
  crate::common::init::init_memory(boot_info);
  pager().print_mem_summary();
  #[cfg(test)]
  {
    info!("Running test harness");
    test_main();
    loop{}
  }
  #[cfg(not(test))]
  {
    {
      unsafe { USERSPACE = Some(Arc::new(RefCell::new(Userspace::new()))) }
      let us = userspace();
      {
        us.in_scheduler_mut_spin(|mut sched| {
          let pid0h = sched.new_elfproc("pid0", crate::inc::PID0);
          match pid0h {
            Ok(pid0h) => {
              sched.register_scheduler(pid0h);
              trace!("Scheduler created with handle {}", pid0h);
            }
            Err(()) => {
              error!("Could not create PID0 KProc");
            }
          }
        });
      }
    }

    info!("entering userspace");

    userspace().enter();

    panic!("left userspace")
  }
}

use core::panic::PanicInfo;

pub fn coredump() -> ! {
  error!("Kernel Core Dumped");
  hlt_cpu!();
}

/// This function is called on panic.
#[panic_handler]
#[no_mangle]
pub fn panic(info: &PanicInfo) -> ! {
  match info.message() {
    Some(s) => error!("Panic occured: {}", s),
    None => error!("Panic had no message"),
  }
  match info.location() {
    Some(s) => error!("Panicked at {}~{}", s.file(), s.line()),
    None => error!("Panic had no stracktrace"),
  }
  #[cfg(test)]
  {
    use crate::bindriver::cpu::qemu::*;
    exit_qemu(QemuExitCode::Failed);
  }
  hlt_cpu!();
}

#[alloc_error_handler]
#[no_mangle]
pub fn alloc_error(layout: core::alloc::Layout) -> ! {
  error!("Allocation Error: {} bytes, CPU halted", layout.size());

  #[cfg(test)]
  {
    use crate::bindriver::cpu::qemu::*;
    exit_qemu(QemuExitCode::Failed);
  }
  hlt_cpu!();
}
