#![feature(abi_x86_interrupt)]
#![feature(alloc)]
#![feature(alloc_error_handler)]
#![feature(allocator_api)]
#![feature(min_const_fn)]
#![feature(lang_items)]
#![feature(const_raw_ptr_to_usize_cast)]
#![feature(asm)]
#![feature(naked_functions)]
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
extern crate bootloader;
extern crate volatile;
extern crate spin;
extern crate uart_16550;
extern crate x86_64;
extern crate slabmalloc;
extern crate raw_cpuid;
#[macro_use]
extern crate alloc;

#[macro_use]
mod common;
#[macro_use]
mod bindriver;
mod version;
mod vmem;
mod process_manager;
mod incproc;

const BOOT_MEMORY_PAGES: usize = 32;

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
  let rsp_enter: usize;
  unsafe { asm!("":"={rsp}"(rsp_enter))};
  // init drivers for core hardware
  bindriver::init();
  vga_println!("BoringOS v{}\n", version::VERSION);
  {
    vga_print!("Initializing VMEM...");
    debug!("Probing existing memory ...");
    debug!("P4CTA: {:#018x}\n", ::vmem::pagetable::P4 as u64);
    debug!("P4PTA: {:#018x}\n", boot_info.p4_table_addr);
    assert!(boot_info.p4_table_addr == ::vmem::pagetable::P4 as u64);
    {
      debug!("Initializing VMEM Slab Allocator...");
      unsafe { PAGER.lock().init_page_store(); }
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
    }
    vga_print_green!("[ OK ]\n");
  }
  breakpoint!();
  {
    vga_print!("Initializing Process Manager...");
    unsafe { USERSPACE = Some(Arc::new(RefCell::new(Userspace::new()))) }
    let us = userspace();
    {
      use ::alloc::string::String;
      let mut sched_mut = us.scheduler_mut().expect("need scheduler to setup PID0");
      match sched_mut.new_kproc(
        String::from("pid0"), 
        ::incproc::pid0)
        {
          Err(_) => {
            vga_print_red!("[FAIL]");
            panic!("could not setup pid0")
          },
          Ok(sched) => {
            sched_mut.register_scheduler(&sched);
            debug!("Scheduler created with handle {}", sched);
          },
        }
    }
    vga_print_green!("[ OK ]\n");
  }
  {
    vga_print!("Initializing Process Environment...");
    //TODO: write penv
    vga_print_red!("[TODO]\n");
  }
  
  dump_stack_addr!();
  debug!("entering userspace");

  userspace().enter();

  unsafe { bindriver::qemu::qemu_shutdown(); }
  panic!("Kernel terminated unexpectedly");
}

use core::panic::PanicInfo;

pub fn coredump() -> ! {
  error!("Kernel Core Dumped");
  vga_print_red!("\n\n===== CORE DUMPED =====\n");
  loop {}
}

/// This function is called on panic.
#[panic_handler]
#[no_mangle]
pub fn panic(info: &PanicInfo) -> ! {
  error!("Panic occured: {}", info);
  vga_print_red!("\n\n===== PANIC OCCURED IN KERNEL =====\n");
  vga_print_red!("{}\n", info);

  loop {}
}

#[alloc_error_handler]
#[no_mangle]
pub fn alloc_error(layout: core::alloc::Layout) -> ! {
  error!("Allocation Error: {} bytes", layout.size());
  vga_print_red!("\n\n===== PANIC OCCURED IN KERNEL =====\n");
  vga_print_red!("\n\n===== MEMORY SUBSYSTEM ERROR  =====\n");
  vga_print_red!("Attempted to allocate {} bytes, vmem subsystem returned error\n", layout.size());

  loop{}
}