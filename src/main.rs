#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![feature(allocator_api)]
#![feature(lang_items)]
#![feature(const_raw_ptr_to_usize_cast)]
#![feature(asm)]
#![feature(naked_functions)]
#![feature(integer_atomics)]
#![feature(panic_info_message)]
#![feature(const_fn)]
#![feature(exclusive_range_pattern)]
#![feature(try_trait)]
#![feature(alloc_prelude)]

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

use self::alloc::sync::Arc;
use self::process_manager::Userspace;
use self::vmem::PageManager;
use core::cell::RefCell;
use linked_list_allocator::LockedHeap;

pub use crate::common::*;

pub static PAGER: KPut<PageManager> = KPut::new(PageManager::new());
#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();
static mut USERSPACE: Option<Arc<RefCell<Userspace>>> = None;

bootloader::entry_point!(kernel_main);

fn kernel_main(boot_info: &'static bootloader::BootInfo) -> ! {
  // init drivers for core hardware
  bindriver::init();
  vga_println!("BoringOS v{}\n", version::VERSION);
  {
    vga_print!("Initializing VMEM...");
    debug!("initializing page mapper with phys mem offset");
    debug!("Probing existing memory ...");
    {
      debug!("Initializing VMEM Slab Allocator...");
      pager().init(boot_info.physical_memory_offset).expect("init on pager failed");
      let mmap = &boot_info.memory_map;
      let mut usable_memory = 0;
      use core::ops::Deref;
      let mmap_entries = mmap.deref();
      for x in 0..mmap_entries.len() {
        let entry = mmap_entries[x];
        let range = entry.range;
        trace!(
          "MMAPE: {:#04x} {:#016x}-{:#016x}: {:?}\n",
          x,
          range.start_addr(),
          range.end_addr(),
          entry.region_type
        );
        use bootloader::bootinfo::MemoryRegionType;
        match entry.region_type {
          MemoryRegionType::Usable => {
            let size = range.end_addr() - range.start_addr();
            trace!(
              "Adding MMAPE {:#04x} to usable memory... {} KiBytes, {} Pages",
              x,
              size / 1024,
              size / 4096
            );
            usable_memory += size;
            unsafe { match pager().add_memory(
              PhysAddr::new(range.start_addr()),
                (size / 4096) as usize - 1,) 
              {
                Ok(_) => {},
                Err(pae) => warn!("could not add memory: {:?}", pae),
              }
            };
          }
          _ => {}
        }
      }
      trace!(
        "Total usable memory: {:16} KiB, {:8} MiB",
        usable_memory / 1024,
        usable_memory / 1024 / 1024
      );
      let free_memory = pager().free_memory();
      debug!(
        "Available memory: {} KiB, {} MiB, {} Pages",
        free_memory / 1024,
        free_memory / 1024 / 1024,
        free_memory / 4096
      );
    }
    vga_print_green!("[ OK ]\n");
  }
  {
    vga_print!("Initializing Process Manager...");
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
            vga_print_red!("[ERR!]");
          }
        }
      });
    }
    vga_print_green!("[ OK ]\n");
  }

  info!("entering userspace");

  userspace().enter();

  panic!("left userspace")
}

use core::panic::PanicInfo;

pub fn coredump() -> ! {
  error!("Kernel Core Dumped");
  vga_print_red!("\n\n===== CORE DUMPED =====\n");
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
  vga_print_red!("\n\n===== PANIC OCCURED IN KERNEL =====\n");
  match info.message() {
    Some(s) => vga_print_red!("{}\n\n", s),
    None => (),
  }
  hlt_cpu!();
}

#[alloc_error_handler]
#[no_mangle]
pub fn alloc_error(layout: core::alloc::Layout) -> ! {
  error!("Allocation Error: {} bytes", layout.size());
  vga_print_red!("\n\n===== PANIC OCCURED IN KERNEL =====\n");
  vga_print_red!("\n\n===== MEMORY SUBSYSTEM ERROR  =====\n");
  vga_print_red!(
    "Attempted to allocate {} bytes, vmem subsystem returned error\n",
    layout.size()
  );

  hlt_cpu!();
}
