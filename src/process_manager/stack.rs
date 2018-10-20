use ::vmem::PhysAddr;
use ::core::cell::RefCell;
use ::alloc::rc::Rc;

pub enum Stack {
  NoStack,
  Stack64K(Rc<RefCell<Stack64K>>),
}

panic_on_drop!(Stack);

impl Stack {
  pub fn new_nostack() -> Stack {
    Stack::NoStack
  }
  pub fn new_64kstack() -> Stack {
    Stack::Stack64K(Stack64K::new())
  }
  pub fn map(&mut self) {
    match self {
      Stack::NoStack => (),
      Stack::Stack64K(s) => (*s).borrow_mut().map(),
    }
  }
  pub fn unmap(&mut self) {
    match self {
      Stack::NoStack => (),
      Stack::Stack64K(s) => (*s).borrow_mut().unmap(),
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
  fn map(&mut self) {
    use ::vmem::mapper::map;
    use ::vmem::mapper::MapType;
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
    panic!("not implemented")
  }
}