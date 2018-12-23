#![feature(abi_x86_interrupt)]
#![feature(alloc)]
#![feature(alloc_error_handler)]
#![feature(allocator_api)]
#![feature(lang_items)]
#![feature(const_raw_ptr_to_usize_cast)]
#![feature(asm)]
#![feature(naked_functions)]
#![feature(integer_atomics)]
#![no_std]
#![no_main]

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate static_assertions;
#[macro_use]
extern crate bitflags;
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

const BOOT_MEMORY_PAGES: usize = 32;

use self::alloc::sync::Arc;
use self::process_manager::Userspace;
use self::vmem::PageManager;
use core::cell::RefCell;
use slabmalloc::SafeZoneAllocator;
use spin::Mutex;

pub use crate::common::*;

pub static PAGER: Mutex<PageManager> = Mutex::new(PageManager {
  first_page_mem: crate::vmem::StaticPage::new(),
  first_range_mem: crate::vmem::StaticPage::new(),
  boot_pages: [crate::vmem::StaticPage::new(); BOOT_MEMORY_PAGES],
  use_boot_memory: true,
  pages: crate::vmem::pagelist::PageListLink::None,
});
#[global_allocator]
static MEM_PROVIDER: SafeZoneAllocator = SafeZoneAllocator::new(&PAGER);
static mut USERSPACE: Option<Arc<RefCell<Userspace>>> = None;

#[no_mangle]
pub extern "C" fn _start(boot_info: &'static bootloader::bootinfo::BootInfo) -> ! {
  // init drivers for core hardware
  bindriver::init();
  vga_println!("BoringOS v{}\n", version::VERSION);
  {
    vga_print!("Initializing VMEM...");
    debug!("Probing existing memory ...");
    debug!("P4CTA: {:#018x}\n", crate::vmem::pagetable::P4 as u64);
    debug!("P4PTA: {:#018x}\n", boot_info.p4_table_addr);
    assert!(boot_info.p4_table_addr == crate::vmem::pagetable::P4 as u64);
    {
      debug!("Initializing VMEM Slab Allocator...");
      unsafe {
        pager().init_page_store();
      }
      debug!("Slab allocator initialized, adding memory");
      let mmap = &boot_info.memory_map;
      let mut usable_memory = 0;
      use core::ops::Deref;
      let mmap_entries = mmap.deref();
      for x in 0..mmap_entries.len() {
        let entry = mmap_entries[x];
        let range = entry.range;
        debug!(
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
            debug!(
              "Adding MMAPE {:#04x} to usable memory... {} KiBytes, {} Pages",
              x,
              size / 1024,
              size / 4096
            );
            usable_memory += size;
            unsafe {
              pager().add_memory(
                crate::vmem::pagelist::PhysAddr::new_unchecked(range.start_addr()),
                (size / 4096) as usize - 1,
              );
              pager().use_boot_memory = false;
            };
          }
          _ => {}
        }
      }
      debug!(
        "Total usable memory: {:16} KiB, {:8} MiB",
        usable_memory / 1024,
        usable_memory / 1024 / 1024
      );
      let free_memory = pager().free_memory();
      debug!(
        "Available memory: {} Bytes, {} KiB, {} MiB, {} Pages",
        free_memory,
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
            sched.register_scheduler(&pid0h);
            debug!("Scheduler created with handle {}", pid0h);
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
  {
    vga_print!("Initializing Process Environment...");
    crate::process_environment::init();
    //TODO: write penv
    vga_print_red!("[ OK ]\n");
  }

  debug!("entering userspace");

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
  error!("Panic occured: {}", info);
  vga_print_red!("\n\n===== PANIC OCCURED IN KERNEL =====\n");
  vga_print_red!("{}\n", info);
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
