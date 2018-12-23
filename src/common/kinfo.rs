use alloc::rc::Rc;
use core::cell::RefCell;
use core::sync::atomic::{AtomicBool, AtomicPtr, AtomicU64, Ordering};
use crate::process_manager::{Memory, MemoryUser, MemoryUserRef};
use crate::vmem::PhysAddr;
use spin::RwLock;

pub struct KernelInfo {
  mapping_task_image_int: AtomicBool,
  current_task_handle_int: AtomicU64,
  current_code_memory_ref_int: AtomicPtr<Rc<RefCell<MemoryUser>>>,
  current_data_memory_ref_int: AtomicPtr<Rc<RefCell<MemoryUser>>>,
  current_bss_memory_ref_int: AtomicPtr<Rc<RefCell<MemoryUser>>>,
  current_stack_memory_ref_int: AtomicPtr<Rc<RefCell<MemoryUser>>>,
  zero_page_addr: AtomicU64,
}
impl KernelInfo {
  const fn new() -> Self {
    KernelInfo {
      mapping_task_image_int: AtomicBool::new(false),
      current_task_handle_int: AtomicU64::new(0),
      current_code_memory_ref_int: AtomicPtr::new(0 as *mut Rc<RefCell<MemoryUser>>),
      current_data_memory_ref_int: AtomicPtr::new(0 as *mut Rc<RefCell<MemoryUser>>),
      current_bss_memory_ref_int: AtomicPtr::new(0 as *mut Rc<RefCell<MemoryUser>>),
      current_stack_memory_ref_int: AtomicPtr::new(0 as *mut Rc<RefCell<MemoryUser>>),
      zero_page_addr: AtomicU64::new(0),
    }
  }
  pub fn mapping_task_image(&mut self, v: Option<bool>) -> bool {
    if v.is_none() {
      self.mapping_task_image_int.load(Ordering::SeqCst)
    } else {
      self
        .mapping_task_image_int
        .store(v.expect("needs to be task image option"), Ordering::SeqCst);
      self.mapping_task_image_int.load(Ordering::SeqCst)
    }
  }
  pub fn get_zero_page_addr(&self) -> PhysAddr {
    let v = self.zero_page_addr.load(Ordering::SeqCst);
    return if v == 0 {
      info!("kernel has no zero page, allocating one");
      let page = crate::alloc_page().expect("must have zero page in kernel");
      debug!("got new zero page, setting KINFO");
      let v = self.zero_page_addr.compare_and_swap(0, page.as_u64(), Ordering::SeqCst);
      if v != page.as_u64() {
        crate::release_page(page);
      }
      page
    } else {
      PhysAddr::new_or_abort(v)
    }
  }
  pub fn get_current_task(&self) -> u64 {
    self.current_task_handle_int.load(Ordering::SeqCst)
  }
  pub fn swap_current_task(&self, v: u64) -> u64 {
    debug!("swapping current task to {:#018x}", v);
    self.current_task_handle_int.swap(v, Ordering::SeqCst)
  }
  pub fn add_code_page(&self, p: PhysAddr) {
    debug!("adding {} to active code memory", p);
    let ptr = self.current_code_memory_ref_int.load(Ordering::SeqCst);
    let mur = MemoryUserRef::from(ptr);
    self.add_page_to_mur(p, mur)
  }
  pub fn add_data_page(&self, p: PhysAddr) {
    debug!("adding {} to active data memory", p);
    let ptr = self.current_data_memory_ref_int.load(Ordering::SeqCst);
    let mur = MemoryUserRef::from(ptr);
    self.add_page_to_mur(p, mur)
  }
  pub fn add_stack_page(&self, p: PhysAddr) {
    debug!("adding {} to active stack memory", p);
    let ptr = self.current_stack_memory_ref_int.load(Ordering::SeqCst);
    let mur = MemoryUserRef::from(ptr);
    self.add_page_to_mur(p, mur)
  }
  fn add_page_to_mur(&self, p: PhysAddr, mur: MemoryUserRef) {
    mur.add_page(p);
  }
  pub fn get_code_memory_ref_size(&self) -> usize {
    let ptr = self.current_code_memory_ref_int.load(Ordering::SeqCst);
    let mur = MemoryUserRef::from(ptr);
    mur.page_count()
  }
  pub fn get_data_memory_ref_size(&self) -> usize {
    let ptr = self.current_data_memory_ref_int.load(Ordering::SeqCst);
    let mur = MemoryUserRef::from(ptr);
    mur.page_count()
  }
  pub fn get_stack_memory_ref_size(&self) -> usize {
    let ptr = self.current_stack_memory_ref_int.load(Ordering::SeqCst);
    let mur = MemoryUserRef::from(ptr);
    mur.page_count()
  }
  pub fn set_memory_ref(&self, v: &Memory) -> MemoryUserRef {
    debug!("setting new active memory");
    match v {
      Memory::Code(s) => MemoryUserRef::from(
        self
          .current_code_memory_ref_int
          .swap(s.into(), Ordering::SeqCst),
      ),
      Memory::User(s) => MemoryUserRef::from(
        self
          .current_data_memory_ref_int
          .swap(s.into(), Ordering::SeqCst),
      ),
      Memory::Stack(s) => MemoryUserRef::from(
        self
          .current_stack_memory_ref_int
          .swap(s.into(), Ordering::SeqCst),
      ),
      Memory::ReadOnly(s) => MemoryUserRef::from(
        self
          .current_bss_memory_ref_int
          .swap(s.into(), Ordering::SeqCst),
      ),
      _ => panic!("tried to assign non-code memory to code memory"),
    }
  }
  pub fn set_code_memory_ref(&self, v: MemoryUserRef) -> MemoryUserRef {
    self.set_memory_ref(&Memory::Code(v))
  }
  pub fn set_data_memory_ref(&self, v: MemoryUserRef) -> MemoryUserRef {
    self.set_memory_ref(&Memory::User(v))
  }
  pub fn set_stack_memory_ref(&self, v: MemoryUserRef) -> MemoryUserRef {
    self.set_memory_ref(&Memory::Stack(v))
  }
  pub fn set_bss_memory_ref(&self, v: MemoryUserRef) -> MemoryUserRef {
    self.set_memory_ref(&Memory::ReadOnly(v))
  }
}
pub static KERNEL_INFO: RwLock<KernelInfo> = RwLock::new(KernelInfo::new());
