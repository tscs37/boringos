
#![feature(panic_handler)]
#![feature(abi_x86_interrupt)]
#![feature(alloc)]
#![feature(alloc_error_handler)]
#![feature(allocator_api)]
#![feature(min_const_fn)]
#![feature(const_raw_ptr_to_usize_cast)]
#![no_std]
#![no_main]

#[macro_use]
extern crate lazy_static;
extern crate bootloader;
extern crate volatile;
extern crate spin;
extern crate uart_16550;
extern crate x86_64;
#[macro_use]
extern crate alloc;
#[macro_use]
mod bindriver;
mod version;
mod vmem;



#[global_allocator]
static mut ALLOCATOR: vmem::malloc::Allocator = vmem::malloc::Allocator{
  boot_memory: spin::Mutex::new(vmem::bmfa::BitmapFrameAllocator{
    mem: [0; vmem::bmfa::BootMemorySize],
    allocbitmap: [0; 4096],
  }),
  slab_memory: None,
  disable_boot_memory: false,
};

#[no_mangle]
pub extern "C" fn _start(boot_info: &'static bootloader::bootinfo::BootInfo) -> ! {
  println!("BoringOS v{}\n", version::VERSION);
  print!("Initializing IDT...");
  bindriver::cpu::idt::init_idt();
  print_green!("[ OK ]\n");
  print!("Loading VMEM Driver...");
  debug!("Attempting to allocate memory...");
  {
    use alloc::boxed::Box;
    let heap_test = Box::new(42);
    debug!("Hello from Heap: {}, Adr={:#016x}", heap_test, heap_test.as_ref() as *const _ as usize);
  }
  debug!("Probing existing memory ...");
  debug!("P4PTA: {:#016x}\n", boot_info.p4_table_addr);
  {
    debug!("Initializing VMEM Slab Allocator...");
    unsafe { ALLOCATOR.init_slab_memory(); }
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
          debug!("Adding MMAPE {:#04x} to usable memory...", x);
          usable_memory += range.end_addr() - range.start_addr();
          unsafe { 
            let res = ALLOCATOR.push_slab(
            range.start_addr() as usize, 
            (range.end_addr() - range.start_addr()) as usize
            );
            match res {
              Ok(size) => { debug!("Added {:08} KiB to memory", size / 1024); }
              Err(msg) => { panic!("Error during Slabbing: {}", msg); }
            }
          };
        },
        _ => {}
      }
    }
    debug!("Total usable memory: {:16} KiB, {:8} MiB", usable_memory / 1024, 
      usable_memory / 1024 / 1024);
    debug!("Disabling Boot Memory...");
    unsafe { ALLOCATOR.disable_boot_memory(); }
    if let Ok(free_memory) = unsafe { ALLOCATOR.free_memory() } {
      debug!("Available memory: {} Bytes, {} KiB, {} MiB, {} Pages", 
        free_memory,
        free_memory / 1024,
        free_memory / 1024 / 1024,
        free_memory / 4096
      );
    } else {
      panic!("could not account free memory after slab init");
    }
    //panic!("EOT");
    debug!("Testing VMEM...");
    let val = alloc::boxed::Box::new(5);
    debug!("VMEM Data: {}", val);
  }
  print_green!("[ OK ]\n");
  print!("Initializing Process Manager & Environment...");
  print_green!("[ OK ]\n");
  print!("Loading InitRamFS...");
  //TODO:
  print_green!("[ OK ]\n");
  print!("Running /bin/init...");

  panic!("Kernel terminated unexpectedly");
}

use core::panic::PanicInfo;

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