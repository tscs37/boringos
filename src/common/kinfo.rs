use alloc::rc::Rc;
use core::cell::RefCell;
use core::sync::atomic::{AtomicBool, AtomicPtr, AtomicU64, Ordering};
use crate::process_manager::{Memory, MemoryUser, MemoryUserRef, TaskHandle};
use crate::vmem::PhysAddr;
use spin::RwLock;
use atomic::Atomic;

pub struct KernelInfo {
  switching_tasks_int: AtomicBool,
  mapping_task_image_int: AtomicBool,
  current_task_handle_int: Atomic<TaskHandle>,
  current_code_memory_ref_int: AtomicPtr<Rc<RefCell<MemoryUser>>>,
  current_data_memory_ref_int: AtomicPtr<Rc<RefCell<MemoryUser>>>,
  current_stack_memory_ref_int: AtomicPtr<Rc<RefCell<MemoryUser>>>,
  zero_page_addr: AtomicU64,
}
impl KernelInfo {
  const fn new() -> Self {
    KernelInfo {
      switching_tasks_int: AtomicBool::new(false),
      mapping_task_image_int: AtomicBool::new(false),
      current_task_handle_int: Atomic::new(TaskHandle::from_c(0)),
      current_code_memory_ref_int: AtomicPtr::new(0 as *mut Rc<RefCell<MemoryUser>>),
      current_data_memory_ref_int: AtomicPtr::new(0 as *mut Rc<RefCell<MemoryUser>>),
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
    if v == 0 {
      info!("kernel has no zero page, allocating one");
      let page = crate::alloc_page().expect("must have zero page in kernel");
      let v = self.zero_page_addr.compare_and_swap(0, page.as_u64(), Ordering::SeqCst);
      if v != page.as_u64() && v != 0 {
        use crate::vmem::pagelist::PagePoolReleaseError;
        match crate::release_page(page) {
          Ok(_) => {},
          Err(pre) => match pre {
            PagePoolReleaseError::PageUntracked => warn!("released untracked page {}", page),
            _ => panic!("error when releasing page: {:?}", pre)
          }
        }
      }
      page
    } else {
      PhysAddr::new_or_abort(v)
    }
  }
  pub fn get_switching_tasks(&self) -> bool {
    self.switching_tasks_int.load(Ordering::SeqCst)
  }
  pub fn set_switching_tasks(&self, cur: bool, new: bool) -> bool {
    self.switching_tasks_int.compare_and_swap(cur, new, Ordering::SeqCst)
  }
  pub fn get_current_task(&self) -> TaskHandle {
    self.current_task_handle_int.load(Ordering::SeqCst)
  }
  pub fn swap_current_task(&self, c: TaskHandle, v: TaskHandle) -> Result<TaskHandle, TaskHandle> {
    trace!("swapping current task to {}", v);
    self.current_task_handle_int.
      compare_exchange(c, v, Ordering::SeqCst, Ordering::SeqCst)
  }
  pub fn add_code_page(&self, p: PhysAddr) {
    trace!("adding {} to active code memory", p);
    let ptr = self.current_code_memory_ref_int.load(Ordering::SeqCst);
    let mur = MemoryUserRef::from(ptr);
    self.add_page_to_mur(p, mur)
  }
  pub fn add_data_page(&self, p: PhysAddr) {
    trace!("adding {} to active data memory", p);
    let ptr = self.current_data_memory_ref_int.load(Ordering::SeqCst);
    let mur = MemoryUserRef::from(ptr);
    self.add_page_to_mur(p, mur)
  }
  pub fn add_stack_page(&self, p: PhysAddr) {
    trace!("adding {} to active stack memory", p);
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
  pub fn set_memory_ref(&self, v: &Memory) -> Memory {
    trace!("setting new active memory: {:?}", v);
    match v {
      Memory::Code(s) => Memory::Code(MemoryUserRef::from(
        self
          .current_code_memory_ref_int
          .swap(s.clone().into(), Ordering::SeqCst),
      )),
      Memory::User(s) => Memory::User(MemoryUserRef::from(
        self
          .current_data_memory_ref_int
          .swap(s.clone().into(), Ordering::SeqCst),
      )),
      Memory::Stack(s) => Memory::Stack(MemoryUserRef::from(
        self
          .current_stack_memory_ref_int
          .swap(s.clone().into(), Ordering::SeqCst),
      )),
      _ => panic!("tried to assign non-code memory to code memory"),
    }
  }
}
pub static KERNEL_INFO: RwLock<KernelInfo> = RwLock::new(KernelInfo::new());
