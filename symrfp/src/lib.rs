#![no_std]

use core::intrinsics::transmute;
pub type SymRf = extern "C" fn (sym_type: u16, sym_name: &str) -> *mut u8;

pub fn convert_argv(symrfp: *const *const u8) -> SymRf {
    unsafe{ transmute(symrfp) }
}

#[repr(u16)]
pub enum SymbolType {
  TestSymbolResolver = 0x0,
  TaskData = 0x1,
  KernelCall = 0x2,
  IPCTarget = 0x3,
}

impl SymbolType {
  pub fn from(d: u16) -> Option<SymbolType> {
    if d > 0x3 {
      return None;
    }
    Some(unsafe { core::mem::transmute(d) })
  }
}
