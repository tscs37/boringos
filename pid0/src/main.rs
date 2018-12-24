#![no_std]
#![feature(start)]
#![feature(alloc_error_handler)]

use symrfp::SymbolType;

#[start]
fn main(argc: isize, symrf: *const *const u8) -> isize {
  let mut sp = uart_16550::SerialPort::new(0x3F8);
  sp.init();
  use core::fmt::Write;
  sp.write_str("PID0 running, testing symbol resolver\n").unwrap(); 
  sp.write_fmt(format_args!("argc={}, Symbol resolver at {:#018x}\n", argc,symrf as u64));
  let symrfp = symrfp::convert_argv(symrf);
  let symrfp_test = unsafe {(symrfp)} (0, "symrfp");
  if symrfp_test as u64 != 42{
    sp.write_fmt(format_args!("symrfp returned {} for symrfp, not 42\n", symrfp_test as u64));
  } else {
    sp.write_str("call into symbol resolver ok, returned 42 for test");
  };
  let bos_initramfs_ptr = symrfp(SymbolType::KernelCall, "bos_initramfs_ptr") as *mut [u8];
  let bos_new_task = symrfp(SymbolType::KernelCall, "bos_new_task");
  let bos_set_taskimage_elf = symrfp(SymbolType::KernelCall, "bos_set_task_image_elf");
  let bos_set_scheduler = symrfp(SymbolType::KernelCall, "bos_set_scheduler");
  let bos_add_interpreter = symrfp(SymbolType::KernelCall, "bos_add_interpreter");
  let bos_load_taskimage_wasm = symrfp(SymbolType::KernelCall, "wasm_set_task_image");
  let bos_yield = symrfp(SymbolType::KernelCall, "yield");
  //TODO: parse initramfs
  //TODO: load scheduler binary
  //TODO: set scheduler
  //TODO: yield
  //TODO: load wasm compiler
  loop{}
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