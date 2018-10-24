use ::vmem::PhysAddr;
use ::core::cell::RefCell;
use ::alloc::rc::Rc;
use ::alloc::vec::Vec;

pub enum Stack {
  NoStack,
  UserStack(Rc<RefCell<StackUser>>),
  KernelStack(Rc<RefCell<StackKernel>>),
}

panic_on_drop!(Stack);

impl Stack {
  pub fn new_nostack() -> Stack {
    Stack::NoStack
  }
  pub fn new_userstack() -> Stack {
    Stack::UserStack(StackUser::new())
  }
  pub fn new_kstack() -> Stack {
    Stack::KernelStack(StackKernel::new())
  }
  pub fn map(&self) {
    match self {
      Stack::NoStack => (),
      Stack::UserStack(s) => (*s).borrow().map(),
      Stack::KernelStack(s) => (*s).borrow().map(),
    }
  }
  pub fn unmap(&self) {
    match self {
      Stack::NoStack => (),
      Stack::UserStack(s) => (*s).borrow().unmap(),
      Stack::KernelStack(s) => (*s).borrow().unmap(),
    }
  }
}

pub struct StackUser {
  pages: Vec<PhysAddr>,
}

impl StackUser {
  fn new() -> Rc<RefCell<StackUser>> {
    Rc::new(RefCell::new(StackUser{
      pages: vec!(),
    }))
  }
  fn map(&self) {
    use ::vmem::mapper::{map,MapType};
    use ::vmem::PhysAddr;
    let base = PhysAddr::new(::vmem::STACK_START as u64)
      .expect("need base for stack map");
    debug!("mapping user stack to {}", base);
    map(base, self.pages.clone(), MapType::Stack);
    debug!("mapping user stack complete");
  }
  fn unmap(&self) {
    let base = PhysAddr::new(::vmem::STACK_START as u64)
      .expect("need base for stack unmap");
    use ::vmem::mapper::{unmap,MapType};
    unmap(base, 16, MapType::Stack);
    panic!("not implemented")
  }
}

pub struct StackKernel {
  pages: Vec<PhysAddr>,
}

impl StackKernel {
  fn new() -> Rc<RefCell<StackKernel>> {
    Rc::new(RefCell::new(StackKernel{
      pages: vec!(),
    }))
  }
  fn map(&self) {
    use ::vmem::mapper::{map,MapType};
    use ::vmem::PhysAddr;
    let base = PhysAddr::new(::vmem::KSTACK_START as u64)
      .expect("need base for kstack map");
    debug!("mapping Kernel Stack to {}", base);
    map(base, self.pages.clone(), MapType::Stack);
    debug!("kstack mapped")
  }
  fn unmap(&self) {
    panic!("kernel stack cannot be unmapped")
  }
}