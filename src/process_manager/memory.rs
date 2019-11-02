use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::vec::Vec;
use core::cell::RefCell;
use crate::vmem::mapper::{map, map_zero, unmap, MapType};
use crate::{PhysAddr, VirtAddr};
use core::convert::TryInto;

#[derive(Clone, Copy)]
pub struct MemoryUserRef {
  internal_ref: *mut Rc<RefCell<MemoryUser>>,
}

impl core::ops::Deref for MemoryUserRef {
  type Target = RefCell<MemoryUser>;

  fn deref(&self) -> &RefCell<MemoryUser> {
    unsafe { (*self.internal_ref).deref() }
  }
}

impl core::fmt::Debug for MemoryUserRef {
  fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
    write!(f, "MUR:{:018x}=>{:?}", self.internal_ref as usize, *self.borrow())
  }
}

impl MemoryUserRef {
  pub fn new() -> Self {
    MemoryUserRef::new_sized(2)
  }
  pub fn new_empty() -> Self {
    MemoryUserRef::new_sized(0)
  }
  pub fn new_sized(n: u8) -> Self {
    let mut pages: Vec<PhysAddr> = Vec::new();
    if n > 0 { for _ in 0..n {
      pages.push(crate::common::alloc_page().expect("could not spawn page for user memory"));
    } }
    let data = Box::new(Rc::new(RefCell::new(MemoryUser {
      pages,
      first_page_offset: 0,
    })));
    let ptr = Box::into_raw(data);
    assert!(ptr as usize != 0, "memory user reference null pointer");
    MemoryUserRef::new_from_ptr(ptr)
  }
  fn new_from_ptr(ptr: *mut Rc<RefCell<MemoryUser>>) -> Self {
    MemoryUserRef{ internal_ref: ptr }
  }
  /// Turns a memory reference into a Copy-on-Write reference. Mooooo!
  pub fn turn_into_cow(&self) -> MemoryUserRef {
    panic!("TODO: implement COW")
  }
  pub fn drop_and_release_memory(&self) {
    panic!("TODO: implement D&R")
  }
  pub fn add_page(&self, pg: PhysAddr) {
    unsafe { (**self.internal_ref).borrow_mut() }.pages.push(pg)
  }
  pub fn page_count(&self) -> usize {
    let mem = unsafe { (**self.internal_ref).borrow_mut() };
    mem.page_count()
  }
}

impl Into<*mut Rc<RefCell<MemoryUser>>> for MemoryUserRef {
  fn into(self) -> *mut Rc<RefCell<MemoryUser>> {
    self.internal_ref
  }
}

impl From<*mut Rc<RefCell<MemoryUser>>> for MemoryUserRef {
  fn from(v: *mut Rc<RefCell<MemoryUser>>) -> MemoryUserRef {
    if (v as u64) == 0 {
      return MemoryUserRef::new_empty();
    } 
    MemoryUserRef::new_from_ptr(v)
  }
}

#[derive(Clone)]
pub struct MemoryKernelRef(*mut Rc<RefCell<MemoryKernel>>);
//panic_on_drop!(MemoryKernelRef);

impl core::ops::Deref for MemoryKernelRef {
  type Target = RefCell<MemoryKernel>;

  fn deref(&self) -> &RefCell<MemoryKernel> {
    unsafe { (*self.0).deref() }
  }
}

impl MemoryKernelRef {
  pub fn new() -> MemoryKernelRef {
    let data = Box::new(Rc::new(RefCell::new(MemoryKernel { pages: vec![] })));
    let ptr = Box::into_raw(data);
    assert!(ptr as usize != 0, "memory kernel reference null pointer");
    MemoryKernelRef(ptr)
  }
  pub fn add_page(&self, pg: PhysAddr) {
    unsafe { (**self.0).borrow_mut() }.pages.push(pg)
  }
}

impl core::fmt::Debug for MemoryKernelRef {
  fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
    write!(f, "MKR:{:018x}=>{:?}", self.0 as usize, *self.borrow())
  }
}

#[derive(Clone, Debug)]
pub enum Memory {
  NoMemory,
  User(MemoryUserRef),
  Code(MemoryUserRef),
  Stack(MemoryUserRef),
  KernelStack(MemoryKernelRef),
}

impl Memory {
  pub fn new_nomemory() -> Memory {
    Memory::NoMemory
  }
  pub fn new_usermemory() -> Memory {
    Memory::User(MemoryUser::new())
  }
  pub fn new_codememory() -> Memory {
    Memory::Code(MemoryUser::new_empty())
  }
  pub fn new_stack() -> Memory {
    Memory::Stack(MemoryUser::new_sized(3))
  }
  pub fn new_kernelstack() -> Memory {
    Memory::KernelStack(MemoryKernel::new())
  }
  pub fn map(&self) {
    match self {
      Memory::NoMemory => (),
      Memory::User(s) => (*s).borrow().map(self.start_address(), MapType::Data),
      Memory::Code(s) => (*s).borrow().map(self.start_address(), MapType::Code),
      Memory::Stack(s) => (*s).borrow().map(self.start_address(), MapType::Stack),
      Memory::KernelStack(s) => (*s).borrow().map(self.start_address(), MapType::Stack),
    }
  }
  // map_rw maps all data into memory but as non-executable read-write
  pub fn map_rw(&self) {
    match self {
      Memory::NoMemory => (),
      Memory::User(s) => (*s).borrow().map(
        self.start_address(),
        MapType::Data,
      ),
      Memory::Code(s) => (*s).borrow().map(
        self.start_address(),
        MapType::Data,
      ),
      Memory::Stack(s) => (*s).borrow().map(
        self.start_address(),
        MapType::Stack,
      ),
      Memory::KernelStack(s) => (*s).borrow().map(
        self.start_address(),
        MapType::Stack,
      ),
    }
  }
  pub fn unmap(&self) {
    match self {
      Memory::NoMemory => (),
      Memory::User(s) => (*s).borrow().unmap(
        VirtAddr::new(crate::vmem::DATA_START.try_into().unwrap()),
        MapType::Data,
      ),
      Memory::Code(s) => (*s).borrow().unmap(
        self.start_address(),
        MapType::Code,
      ),
      Memory::Stack(s) => (*s).borrow().unmap(
        self.start_address(),
        MapType::Stack,
      ),
      Memory::KernelStack(s) => (*s).borrow().unmap(
        self.start_address(),
        MapType::Stack,
      ),
    }
  }
  pub fn start_address(&self) -> VirtAddr {
    let ret = match self {
      Memory::NoMemory => VirtAddr::new(0),
      Memory::User(_) => VirtAddr::new(crate::vmem::DATA_START.try_into().unwrap()),
      Memory::Code(_) => VirtAddr::new(crate::vmem::CODE_START.try_into().unwrap()),
      Memory::Stack(_) => VirtAddr::new(crate::vmem::STACK_START.try_into().unwrap()),
      Memory::KernelStack(_) => VirtAddr::new(crate::vmem::KSTACK_START.try_into().unwrap()),
    };
    let ret = ret + self.get_first_page_offset() as u64;
    ret
  }
  pub fn get_first_page_offset(&self) -> u32 {
    match self {
      Memory::NoMemory => 0,
      Memory::User(s) => (*s).borrow().offset(),
      Memory::Code(s) => (*s).borrow().offset(),
      Memory::Stack(_) => 0, // Stacks don't skip
      Memory::KernelStack(_) => 0,
    }
  }
  pub fn set_first_page_offset(&self, offset: u32) {
    let cur_offset = self.get_first_page_offset();
    if cur_offset != 0 {
      // someone wrote bad code, kill the kernel
      panic!(
        "kernel attempted to set first_page_offset twice: 1. {:#06x}, 2. {:#06x}",
        cur_offset, offset
      )
    }
    debug!("setting first_page_offset={:#06x}", offset);
    match self {
      Memory::NoMemory => (),
      Memory::User(s) => (*s).borrow_mut().set_offset(offset),
      Memory::Code(s) => (*s).borrow_mut().set_offset(offset),
      Memory::Stack(_) => panic!("kernel tried to set offset on stack memory"),
      Memory::KernelStack(_) => panic!("kernel tried to set offset on kernel stack memory"),
    }
  }
  pub fn page_count(&self) -> usize {
    match self {
      Memory::NoMemory => 0,
      Memory::User(s) => (*s).borrow().page_count(),
      Memory::Code(s) => (*s).borrow().page_count(),
      Memory::Stack(s) => (*s).borrow().page_count(),
      Memory::KernelStack(s) => (*s).borrow().page_count(),
    }
  }
}

pub struct MemoryUser {
  pages: Vec<PhysAddr>,
  // pages to place the memory from the actual start of the section
  // bss memory relies on this
  first_page_offset: u32,
}

impl core::fmt::Debug for MemoryUser {
  fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
    write!(f, "MemoryUser {{ pages: {}, zpo: {} }}", self.pages.len(), self.first_page_offset)
  }
}

impl MemoryUser {
  fn new() -> MemoryUserRef {
    MemoryUserRef::new()
  }
  fn new_sized(n: u8) -> MemoryUserRef {
    MemoryUserRef::new_sized(n)
  }
  fn new_empty() -> MemoryUserRef {
    MemoryUserRef::new_empty()
  }
  fn map(&self, base: VirtAddr, t: MapType) {
    if self.pages.len() == 0 {
      return;
    }
    if self.first_page_offset != 0 {
      trace!("pre-mapping zero pages");
      map_zero(base, self.first_page_offset);
    }
    trace!("mapping user memory to {:?} ({:?})", base, t);
    let adj_base = base + (self.first_page_offset as usize) * crate::vmem::PAGE_SIZE;
    map(
      adj_base,
      self.pages.clone(),
      t,
    );
  }
  fn unmap(&self, base: VirtAddr, t: MapType) {
    if self.pages.len() == 0 {
      return;
    }
    if self.first_page_offset != 0 {
      trace!("pre-unmapping zero pages");
      unmap(base, self.first_page_offset as usize, MapType::Zero);
    }
    trace!("unmapping user memory at {:?} ({:?})", base, t);
    let adj_base = base + (self.first_page_offset as usize) * crate::vmem::PAGE_SIZE;
    unmap(adj_base, self.pages.len(), t);
  }
  pub fn set_offset(&mut self, offset: u32) {
    self.first_page_offset = offset
  }
  pub fn offset(&self) -> u32 {
    self.first_page_offset
  }
  fn page_count(&self) -> usize {
    self.pages.len() + self.first_page_offset as usize
  }
}

#[derive(Debug)]
pub struct MemoryKernel {
  pages: Vec<PhysAddr>,
}

impl MemoryKernel {
  fn new() -> MemoryKernelRef {
    MemoryKernelRef::new()
  }
  fn map(&self, base: VirtAddr, t: MapType) {
    if self.pages.len() == 0 {
      return;
    }
    info!("mapping Kernel Memory to {:?}", base);
    map(base, self.pages.clone(), t);
  }
  fn unmap(&self, _base: VirtAddr, _t: MapType) {
    panic!("kernel memory cannot be unmapped")
  }
  fn page_count(&self) -> usize {
    self.pages.len()
  }
}
