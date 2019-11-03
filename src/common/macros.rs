
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


macro_rules! page_range {
  ($t:ident) => { {
    use $crate::vmem::*;
    let start = concat_idents!($t, _START);
    let end = concat_idents!($t, _END);
    page_range!(start, end)
  } };
  ($t:ident + 1) => { {
    use $crate::vmem::*;
    let start = concat_idents!($t, _START);
    let end = concat_idents!($t, _END);
    page_range!(start, end + $crate::vmem::PAGE_SIZE)
  } };
  ($start:expr, $end:expr) => { {
    use core::convert::TryInto;
    VirtAddr::new($start.try_into().unwrap())
    ..
    VirtAddr::new($end.try_into().unwrap())
  } };
}
