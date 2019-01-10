#![no_std]
#![feature(start)]
#![feature(alloc_error_handler)]
#![feature(alloc)]

#[macro_use]
extern crate symrfp;

extern crate ralloc;

#[global_allocator]
static ALLOCATOR: ralloc::Allocator = ralloc::Allocator{};

extern crate alloc;

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
  sp.write_str("PID0 running, testing symbol resolver\n");
  let symrfp_test: &u64 = get_symbol(SymbolType::TestSymbolResolver, "symrfp");
  if *symrfp_test != 42{
    sp.write_fmt(format_args!("symrfp returned {} for symrfp, not 42\n", symrfp_test as &u64));
    panic!();
  } else {
    sp.write_str("call into symbol resolver ok, returned 42 for test\n");
  };
  sp.write_str("loading symbols for system setup\n");
  {
    // get some more pages
    import_symbol!(bos_raise_page_limit, fn(u16) -> u64);
    sp.write_fmt(format_args!("New pagelimit: {}\n", bos_raise_page_limit(1028)));
    sp.write_fmt(format_args!("New pagelimit: {}\n", bos_raise_page_limit(1028)));
  }
  import_symbol!(bos_log_debug, fn(&str));
  {
    bos_log_debug("testing memory allocator");
    use alloc::alloc::{alloc, dealloc, Layout};
    unsafe{
      let layout = Layout::new::<u16>();
      let ptr = alloc(layout);
      bos_log_debug(&alloc::fmt::format(format_args!("got ptr: {:#018x}", ptr as u64)));
      *(ptr as *mut u16) = 42;
      assert_eq!(*(ptr as *mut u16), 42);
      dealloc(ptr, layout);
    }
    bos_log_debug("memory allocator ok!");
  }
  import_symbol!(bos_log_debug_fmt, fn(core::fmt::Arguments));
  import_symbol!(bos_spawn_task, fn() -> u128);
  let scheduler_th = bos_spawn_task();
  bos_log_debug_fmt(format_args!("scheduler task handle: {:#018x}", scheduler_th));
  import_symbol!(bos_yield, fn(u128));
  sp.write_str("loaded smybols, setting up scheduler...\n");
  //TODO: parse initramfs
  //TODO: load scheduler binary
  //TODO: set scheduler
  //TODO: yield
  //TODO: load wasm compiler
  loop {
    bos_yield(scheduler_th);
    bos_log_debug("returned from scheduler, yielding again.");
    panic!()
  }
}

use core::panic::PanicInfo;

/// This function is called on panic.
#[panic_handler]
#[no_mangle]
pub fn panic(info: &PanicInfo) -> ! {
  import_symbol!(bos_log_error_fmt, fn(core::fmt::Arguments));
  bos_log_error_fmt(format_args!("Panic: {:?}", info));
  loop{}
}

#[alloc_error_handler]
#[no_mangle]
pub fn alloc_error(layout: core::alloc::Layout) -> ! {
  import_symbol!(bos_log_error_fmt, fn(core::fmt::Arguments));
  bos_log_error_fmt(format_args!("AllocError: {:?}", layout));
  loop{}
}