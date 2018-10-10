
#![feature(abi_x86_interrupt)]
#![feature(alloc)]
#![feature(alloc_error_handler)]
#![feature(allocator_api)]
#![feature(min_const_fn)]
#![feature(lang_items)]
#![feature(const_raw_ptr_to_usize_cast)]
#![feature(asm)]
#![no_std]
#![no_main]

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate static_assertions;
extern crate bootloader;
extern crate volatile;
extern crate spin;
extern crate uart_16550;
extern crate x86_64;
extern crate slabmalloc;
extern crate alloc;

#[macro_use]
mod bindriver;
#[macro_use]
mod common;
mod version;
mod vmem;
mod process_manager;
mod incproc;

const BOOT_MEMORY_PAGES: usize = 8;

use slabmalloc::SafeZoneAllocator;
use spin::Mutex;
use vmem::PageManager;
use ::process_manager::Userspace;
use ::core::cell::RefCell;
use ::alloc::sync::Arc;

pub use ::common::*;

static PAGER: Mutex<PageManager> = Mutex::new(PageManager {
  first_page_mem: ::vmem::StaticPage::new(),
  first_range_mem: ::vmem::StaticPage::new(),
  boot_pages: [::vmem::StaticPage::new(); BOOT_MEMORY_PAGES],
  use_boot_memory: true,
  pages: ::vmem::pagelist::PageListLink::None,
});
#[global_allocator]
static MEM_PROVIDER: SafeZoneAllocator = SafeZoneAllocator::new(&PAGER);
static mut USERSPACE: Option<Arc<RefCell<Userspace>>> = None;

#[no_mangle]
pub extern "C" fn _start(boot_info: &'static bootloader::bootinfo::BootInfo) -> ! {
  println!("BoringOS v{}\n", version::VERSION);
  print!("Initializing IDT...");
  bindriver::cpu::idt::init_idt();
  print_green!("[ OK ]\n");
  print!("Loading VMEM Driver...");
  debug!("Probing existing memory ...");
  debug!("P4PTA: {:#016x}\n", boot_info.p4_table_addr);
  {
    debug!("Initializing VMEM Slab Allocator...");
    unsafe { PAGER.lock().init_page_store(); }
    {
        debug!("Attempting to allocate memory...");
        {
          use alloc::boxed::Box;
          let heap_test = Box::new(42);
          debug!("Hello from Heap: {}, Adr={:#016x}", heap_test, heap_test.as_ref() as *const _ as usize);
        }
    }
    debug!("Slab allocator initialized, adding memory");
    let mmap = &boot_info.memory_map;
    let mut usable_memory = 0;
    use core::ops::Deref;
    let mmap_entries = mmap.deref();
    for x in 0..mmap_entries.len() {
      let entry = mmap_entries[x];
      let range = entry.range;
      debug!("MMAPE: {:#04x} {:#016x}-{:#016x}: {:?}\n", x,
          range.start_addr(), range.end_addr(), entry.region_type);
      use bootloader::bootinfo::MemoryRegionType;
      match entry.region_type {
        MemoryRegionType::Usable => { 
          let size = range.end_addr() - range.start_addr();
          debug!("Adding MMAPE {:#04x} to usable memory... {} KiBytes, {} Pages", x, size / 1024, size / 4096);
          usable_memory += size;
          unsafe { 
            PAGER.lock().add_memory(
              ::vmem::pagelist::PhysAddr::new_unchecked(range.start_addr()),
              (size / 4096) as usize - 1
            );
            PAGER.lock().use_boot_memory = false;
          };
        },
        _ => {}
      }
    }
    debug!("Total usable memory: {:16} KiB, {:8} MiB", usable_memory / 1024, 
      usable_memory / 1024 / 1024);
    let free_memory = PAGER.lock().free_memory();
    debug!("Available memory: {} Bytes, {} KiB, {} MiB, {} Pages", 
      free_memory,
      free_memory / 1024,
      free_memory / 1024 / 1024,
      free_memory / 4096
    );
    debug!("Testing VMEM...");
    let val = alloc::boxed::Box::new(5);
    debug!("VMEM Data: {}", val);
  }
  print_green!("[ OK ]\n");
  print!("Initializing Process Manager...");
  unsafe { USERSPACE = Some(Arc::new(RefCell::new(Userspace::new()))) }
  let us = userspace();
  {
    use ::process_manager::{Handle, ProcessHandle};
    use ::alloc::string::String;
    match us.scheduler().new_kproc(
      &ProcessHandle::from(Handle::from(0)), 
      &ProcessHandle::from(Handle::from(0)),
      String::from("pid0"), 
      ::incproc::pid0)
      {
        Err(_) => panic!("could not setup pid0"),
        Ok(_) => (),
      }
  }
  print_green!("[ OK ]\n");
  print!("Initializing Process Environment...");
  print_green!("[ OK ]\n");
  println!("Yielding to scheduler...");

  us.enter();

  unsafe { bindriver::qemu::qemu_shutdown(); }
  panic!("Kernel terminated unexpectedly");
}

use core::panic::PanicInfo;

pub fn coredump() -> ! {
  print_red!("\n\n===== CORE DUMPED =====\n");
  loop {}
}

/// This function is called on panic.
#[panic_handler]
#[no_mangle]
pub fn panic(info: &PanicInfo) -> ! {
  print_red!("\n\n===== PANIC OCCURED IN KERNEL =====\n");
  print_red!("{}\n", info);

  loop {}
}

#[alloc_error_handler]
#[no_mangle]
pub fn alloc_error(layout: core::alloc::Layout) -> ! {
  print_red!("\n\n===== PANIC OCCURED IN KERNEL =====\n");
  print_red!("\n\n===== MEMORY SUBSYSTEM ERROR  =====\n");
  print_red!("Attempted to allocate {} bytes, vmem subsystem returned error\n", layout.size());

  loop{}
}