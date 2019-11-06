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
#![feature(concat_idents)]

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

pub use crate::common::*;

pub static PAGER: KPut<PageManager> = KPut::new(PageManager::new());
#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();
static mut USERSPACE: Option<Arc<RefCell<Userspace>>> = None;

bootloader::entry_point!(kernel_main);

fn kernel_main(boot_info: &'static bootloader::BootInfo) -> ! {
  // init drivers for core hardware
  bindriver::init();
  info!("BoringOS v{}\n", version::VERSION);
  {
    debug!("Probing existing memory ...");
    {
      debug!("Initializing VMEM Allocator...");
      pager().init(VirtAddr::new(boot_info.physical_memory_offset)).expect("init on pager failed");
      {
        let start = vmem::KHEAP_ALLOC;
        let size = vmem::KHEAP_END - vmem:: KHEAP_START;
        debug!("initializing allocator from {:#018x} with {} pages", start, size / 4096);
        unsafe { ALLOCATOR.init(start, size) };
      }
      debug!("loading memory map");
      let mmap = &boot_info.memory_map;
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
            let size = (range.end_addr() - range.start_addr()) / 4096;
            trace!(
              "Adding MMAPE {:#04x} to usable memory... {} KiBytes, {} Pages",
              x,
              size * 4,
              size
            );
            use crate::vmem::pagelist::pagelist_ng::PageMap;
            use alloc::alloc::{Alloc, Global};
            let layout = PageManager::pagemap_layout();
            let mut rem_pages: u64 = size;
            let mut total_added_pages: u64 = 0;
            while rem_pages > 0 { 
              let ptr = unsafe{(Global{}).alloc_zeroed(layout)}
                .expect("could not allocate memory for pagemap");
              let ptr: *mut PageMap = ptr.cast().as_ptr();
              let added_pages = unsafe { match pager().add_memory(
                ptr,
                PhysAddr::new(range.start_addr() + (total_added_pages as u64 * 4096)),
                  rem_pages.try_into().unwrap()) 
                {
                  Ok(v) => v,
                  Err(pae) => panic!("could not add memory: {:?}", pae),
                }
              };
              rem_pages -= added_pages;
              total_added_pages += added_pages;
              assert!((rem_pages as u64) < size, "rem_pages has overflown");
            }
          }
          _ => {}
        }
      }
      pager().print_mem_summary();
    }
  }
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
  hlt_cpu!();
}

#[alloc_error_handler]
#[no_mangle]
pub fn alloc_error(layout: core::alloc::Layout) -> ! {
  error!("Allocation Error: {} bytes, CPU halted", layout.size());

  hlt_cpu!();
}
