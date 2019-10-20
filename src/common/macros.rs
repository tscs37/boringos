
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
