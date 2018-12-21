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
    if self.pages.len() == 0 { return; }
    use ::vmem::mapper::{map,MapType};
    use ::vmem::PhysAddr;
    debug!("mapping user memory");
    let base = PhysAddr::new(::vmem::STACK_START as u64)
      .expect("need base for memory map");
    debug!("mapping user memory to {}", base);
    map(base, self.pages.clone(), MapType::Stack);
    debug!("mapping user memory complete");
  }
  fn unmap(&self) {
    let base = PhysAddr::new(::vmem::STACK_START as u64)
      .expect("need base for memory unmap");
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
    if self.pages.len() == 0 { return; }
    use ::vmem::mapper::{map,MapType};
    use ::vmem::PhysAddr;
    debug!("mapping kernel memory");
    let base = PhysAddr::new(::vmem::KSTACK_START as u64)
      .expect("need base for kernel memory map");
    debug!("mapping Kernel Memory to {}", base);
    map(base, self.pages.clone(), MapType::Stack);
    debug!("kernel memory mapped")
  }
  fn unmap(&self) {
    panic!("kernel memory cannot be unmapped")
  }
}