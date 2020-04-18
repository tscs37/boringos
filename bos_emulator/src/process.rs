
use crate::*;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;
use wasmtime_jit::{Compiler, instantiate, NullResolver};
use wasmtime_runtime::Export;
use cranelift_codegen::settings;
use cranelift_codegen::settings::Configurable;

pub struct ProcessImage {
  code_image: Vec<u8>,
}

impl ProcessImage {
  pub fn new_from_file<S>(path: S) -> Self where S: Into<String> {
    let code_image = std::fs::read(path.into()).expect("code image not found");
    ProcessImage {
      code_image,
    }
  }
  /// runs process until yield()
  pub fn run(&self, symrfp: symrfp::SymRf) -> Option<i64> {
    let mut settings_builder = settings::builder();
    info!("Setting up JIT");
    settings_builder.enable("enable_verifier").unwrap();
    settings_builder.enable("is_pic").unwrap();
    settings_builder.enable("enable_float").unwrap();
    settings_builder.enable("probestack_enabled").unwrap();
    let isa_builder = cranelift_native::builder().unwrap_or_else(|_| {
      panic!("unsupported ISA");
    });
    let isa = isa_builder.finish(settings::Flags::new(settings_builder));
    let mut resolver = NullResolver {};
    let mut compiler = Compiler::new(isa, wasmtime_jit::CompilationStrategy::Auto);
    let global_exports = Rc::new(RefCell::new(HashMap::new()));
    let instance = instantiate(
      &mut compiler, &self.code_image, &mut resolver, 
      global_exports, false);
    let mut instance = instance.unwrap();
    let init_export: Export = instance.lookup("__bos_start__").expect("module did not export __bos_start__");
    match init_export {
      Export::Function{ address, vmctx, signature } => {
        let signature: cranelift_codegen::ir::Signature = signature;
        info!("Signature: {}", signature);
        let vmctx: *mut wasmtime_runtime::VMContext = vmctx;
        info!("Initializing Environment and Runtime");
        wasmtime_runtime::wasmtime_init_eager();
        wasmtime_runtime::wasmtime_init_finish(unsafe{&mut *vmctx});
        let address: *const wasmtime_runtime::VMFunctionBody = address;
        trace!("Got function, calling");
        let mut ret_buffer: [u8; 32] = [0xFF; 32];
        unsafe{wasmtime_runtime::wasmtime_call_trampoline(
          vmctx, address, ret_buffer.as_mut_ptr()
        )}.unwrap();
        use std::convert::TryInto;
        let (mut ret, mut stp) = ret_buffer.split_at(std::mem::size_of::<i64>());
        let ret: i64 = i64::from_le_bytes(ret.try_into().unwrap());
        trace!("Return value: {:?}", ret_buffer);
        return Some(ret);
      },
      _ => { panic!("expected start to be a function") }
    }
    panic!("not matching init export");
  }
}