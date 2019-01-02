#![no_std]
#![feature(start)]
#![feature(alloc_error_handler)]
#![feature(alloc)]

#[macro_use]
extern crate symrfp;
extern crate ralloc;

use symrfp::{SymbolType, get_symbol};

extern "C" fn sighandler(sig: u64, id: u64) -> u64 {
  import_symbol!(bos_sig_handle, fn(u64, u64, u64));
  bos_sig_handle(sig, id, 0);
  0
}

#[start]
fn main(argc: isize, symrf: *const *const u8) -> isize {
  // Required prologue for BoringOS, sets the symbol resolver globally
  symrfp::_init(symrf);
  import_symbol!(bos_set_sig_handler, fn(u64));
  bos_set_sig_handler(sighandler as *mut u8 as u64);
  task();
  loop{}
}

fn task() {
  let mut sp = uart_16550::SerialPort::new(0x3F8);
  sp.init();
  use core::fmt::Write;
  sp.write_str("PID0 running, testing symbol resolver\n").unwrap();
  let symrfp_test: &u64 = get_symbol(SymbolType::TestSymbolResolver, "symrfp");
  if *symrfp_test != 42{
    sp.write_fmt(format_args!("symrfp returned {} for symrfp, not 42\n", symrfp_test as &u64));
  } else {
    sp.write_str("call into symbol resolver ok, returned 42 for test\n");
  };
  sp.write_str("loading symbols for system setup\n");
  {
    // get some more pages
    import_symbol!(bos_raise_page_limit, fn(u16) -> u64);
    sp.write_fmt(format_args!("New pagelimit: {}\n", bos_raise_page_limit(1028)));
    // what?
    //import_symbol!(bos_raise_page_limit, fn(u16) -> u64);
    sp.write_fmt(format_args!("New pagelimit: {}\n", bos_raise_page_limit(1028)));
  }
  /*let bos_get_initramfs = get_symbol(SymbolType::KernelCall, "bos_get_initramfs") ;
  let bos_new_task = get_symbol(SymbolType::KernelCall, "bos_new_task");
  let bos_set_scheduler = get_symbol(SymbolType::KernelCall, "bos_set_scheduler");
  let bos_add_interpreter = get_symbol(SymbolType::KernelCall, "bos_add_interpreter");
  let bos_load_taskimage_wasm = get_symbol(SymbolType::KernelCall, "wasm_set_task_image");
  let bos_yield = get_symbol(SymbolType::KernelCall, "yield");*/
  sp.write_str("loaded smybols, setting up scheduler...\n");
  //TODO: parse initramfs
  //TODO: load scheduler binary
  //TODO: set scheduler
  //TODO: yield
  //TODO: load wasm compiler
}

use core::panic::PanicInfo;

/// This function is called on panic.
#[panic_handler]
#[no_mangle]
pub fn panic(info: &PanicInfo) -> ! {
  loop{}
}

#[alloc_error_handler]
#[no_mangle]
pub fn alloc_error(layout: core::alloc::Layout) -> ! {
  loop{}
}