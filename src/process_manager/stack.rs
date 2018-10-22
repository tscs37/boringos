use ::vmem::PhysAddr;
use ::core::cell::RefCell;
use ::alloc::rc::Rc;
use ::alloc::vec::Vec;

pub enum Stack {
  NoStack,
  Stack64K(Rc<RefCell<Stack64K>>),
  KernelStack(Rc<RefCell<StackKernel>>),
}

panic_on_drop!(Stack);

impl Stack {
  pub fn new_nostack() -> Stack {
    Stack::NoStack
  }
  pub fn new_64kstack() -> Stack {
    Stack::Stack64K(Stack64K::new())
  }
  pub fn new_kstack() -> Stack {
    Stack::KernelStack(StackKernel::new())
  }
  pub fn map(&self) {
    match self {
      Stack::NoStack => (),
      Stack::Stack64K(s) => (*s).borrow().map(),
      Stack::KernelStack(s) => (*s).borrow().map(),
    }
  }
  pub fn unmap(&self) {
    match self {
      Stack::NoStack => (),
      Stack::Stack64K(s) => (*s).borrow().unmap(),
      Stack::KernelStack(s) => (*s).borrow().unmap(),
    }
  }
}

pub struct Stack64K {
  pages: [PhysAddr; 16],
}

impl Stack64K {
  fn new() -> Rc<RefCell<Stack64K>> {
    let mut pages: [PhysAddr; 16] = unsafe { ::core::mem::uninitialized() };
    for x in 0..16 {
      pages[x] = ::alloc_page().expect("need pages for Stack64K")
    }
    Rc::new(RefCell::new(Stack64K{
      pages: pages,
    }))
  }
  fn map(&self) {
    use ::vmem::mapper::{map,MapType};
    use ::vmem::PhysAddr;
    use ::alloc::vec::Vec;
    let base = PhysAddr::new(::vmem::STACK_START as u64)
      .expect("need base for stack map");
    debug!("mapping 64K Stack to {}", base);
    let mut pages = Vec::new();
    pages.extend_from_slice(&self.pages);
    map(base, pages, MapType::Stack);
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
  }
  fn unmap(&self) {
    panic!("kernel stack cannot be unmapped")
  }
}