use ::vmem::PhysAddr;
use ::core::cell::RefCell;
use ::alloc::rc::Rc;
use ::alloc::vec::Vec;

#[derive(Clone)]
pub enum Stack {
  NoStack,
  User(Rc<RefCell<MemoryUser>>),
  Kernel(Rc<RefCell<MemoryKernel>>),
}

#[derive(Clone)]
pub enum Memory {
  NoMemory,
  User(Rc<RefCell<MemoryUser>>),
}

panic_on_drop!(Stack);

impl Stack {
  pub fn new_nostack() -> Stack {
    Stack::NoStack
  }
  pub fn new_userstack() -> Stack {
    Stack::User(MemoryUser::new())
  }
  pub fn new_kstack() -> Stack {
    Stack::Kernel(MemoryKernel::new())
  }
  pub fn map(&self) {
    match self {
      Stack::NoStack => (),
      Stack::User(s) => (*s).borrow().map(),
      Stack::Kernel(s) => (*s).borrow().map(),
    }
  }
  pub fn unmap(&self) {
    match self {
      Stack::NoStack => (),
      Stack::User(s) => (*s).borrow().unmap(),
      Stack::Kernel(s) => (*s).borrow().unmap(),
    }
  }
}

impl Memory {
  pub fn new_nomemory() -> Memory {
    Memory::NoMemory
  }
  pub fn new_usermemory() -> Memory {
    Memory::User(MemoryUser::new())
  }
  pub fn map(&self) {
    match self {
      Memory::NoMemory => (),
      Memory::User(s) => (*s).borrow().map(),
    }
  }
  pub fn unmap(&self) {
    match self {
      Memory::NoMemory => (),
      Memory::User(s) => (*s).borrow().map(),
    }
  }
}

pub struct MemoryUser {
  pages: Vec<PhysAddr>,
}

impl MemoryUser {
  fn new() -> Rc<RefCell<MemoryUser>> {
    Rc::new(RefCell::new(MemoryUser{
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

pub struct MemoryKernel {
  pages: Vec<PhysAddr>,
}

impl MemoryKernel {
  fn new() -> Rc<RefCell<MemoryKernel>> {
    Rc::new(RefCell::new(MemoryKernel{
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