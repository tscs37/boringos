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
  pub fn map(&self) {
    match self {
      Stack::NoStack => (),
      Stack::Stack64K(s) => (*s).borrow().map(),
    }
  }
  pub fn unmap(&self) {
    match self {
      Stack::NoStack => (),
      Stack::Stack64K(s) => (*s).borrow().map(),
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
  fn map(&self) {}
  fn unmap(&self) {}
}