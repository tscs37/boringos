macro_rules! proc_yield {
  () => {
    ::common::yield_to(0,0);
  }
}
macro_rules! ipc_call {
  ($fnc:ident, $data:expr) => { {
    debug!("called function {} with payload {}", stringify!($fnc), stringify!($data));
    0
  } };
  ($fnc:ident) => { {
    debug!("called function {}", stringify!($fnc));
    0
  } }
}

#[allow(unused_macros)]
macro_rules! ipc_return {
  ($data:expr) => { {
    let ipc_data = $data;
    debug!("Returning value {:#018x}", ipc_data);
    loop{}
  } }
}

#[allow(unused_macros)]
macro_rules! ipc_error {
  ($code:expr) => { {
    let ipc_err = $code;
    debug!("Returning error {:#x}", ipc_err);
    loop{}
  } }
}

#[allow(unused_macros)]
macro_rules! breakpoint {
  () => {
    ::x86_64::instructions::int3();
  };
}

macro_rules! dump_stack_addr {
  () => { debug!("Stack at {:#018x}", stack_addr!()) }
}

macro_rules! stack_addr {
  () => { {
      let rsp: usize;
      unsafe { asm!("" : "={rsp}"(rsp)); };
      rsp
  } }
}

macro_rules! panic_on_drop {
  ($type_name:ident) => {
    impl Drop for $type_name {
      fn drop(&mut self) {
        panic!("Resource {} marked as panic_on_drop but was dropped", stringify!($type_name));
      }
    }
  }
}

macro_rules! hlt_cpu {
  () => {
    loop {
      hlt_once!();
    }
  }
}

macro_rules! hlt_once {
  () => {
    ::x86_64::instructions::hlt();
  };
}