#![no_std]
#![feature(concat_idents)]

use core::mem::transmute;
pub type SymRf = extern "C" fn (sym_type: u16, sym_name: &str) -> *const u8;

fn convert_argv(symrfp: *const *const u8) -> SymRf {
    unsafe{ transmute(symrfp) }
}

pub fn _init(symrfp: *const *const u8) {
  let s = convert_argv(symrfp);
  unsafe { _SYMRFP = Some(s) };
}

static mut _SYMRFP: Option<SymRf> = None;

pub fn get_symbol<U>(st: SymbolType, sym_name: &str) -> &U {
  //TODO: cache symbol access
  let sym = unsafe{_SYMRFP}.expect("need symbol resolver initialized")(st as u16, sym_name);
  unsafe{ transmute(&sym) }
}

#[macro_export]
macro_rules! import_symbol {
  ($name:ident, $t:ty) => {
    import_symbol!($name as $name, $t);
  };
  ($name:ident as $actualname:ident, $t:ty) => {
    let $actualname: &$t = $crate::get_symbol($crate::SymbolType::IPC, stringify!($name));
    let $actualname: $t = *$actualname;
  };
}

#[repr(u16)]
#[derive(Debug, Copy, Clone)]
pub enum SymbolType {
  TestSymbolResolver = 0x0,
  TaskData = 0x1,
  IPC = 0x3,
}

impl SymbolType {
  pub fn from(d: u16) -> Option<SymbolType> {
    if d > 0x3 {
      return None;
    }
    Some(unsafe { transmute(d) })
  }
}
