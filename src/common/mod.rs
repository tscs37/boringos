
use ::USERSPACE;
use ::process_manager::Userspace;
use ::process_manager::TaskHandle;
use ::core::cell::Ref;

#[allow(dead_code)]

pub fn userspace<'a>() -> Ref<'static, Userspace> {
  unsafe {
    (*(USERSPACE.as_mut().expect("userspace required"))).borrow()
  }
}

pub fn yield_to(th: Option<TaskHandle>) {
  userspace().yield_to(th)
}

pub fn alloc_page() -> Option<::vmem::PhysAddr> {
  unsafe { ::PAGER.lock().alloc_page() }
}

pub fn release_page(pa: ::vmem::PhysAddr) {
  unsafe { ::PAGER.lock().free_page(pa) }
}

macro_rules! panic_on_drop {
  ($type_name:ident) => {
    impl Drop for $type_name {
      fn drop(&mut self) {
        panic!("Resource $type_name marked as panic_on_drop but was dropped");
      }
    }
  }
}