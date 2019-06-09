pub mod pagelist;
pub mod pagetable;
pub mod mapper;
pub mod faulth;

use slabmalloc::ObjectPage;
use core::convert::TryFrom;
use core::convert::TryInto;
use core::option::NoneError;
pub use crate::vmem::pagelist::PhysAddr;
pub use crate::vmem::pagetable::PAGE_ADDR_FILTER;

pub const PAGE_SIZE: usize = 4096;

const BOOT_MEMORY_PAGES: u16 = 32;

pub const PAGE_TABLE_LO: usize = 0xffff_ff80_0000_0000;
pub const KSTACK_GUARD: usize  = 0xffff_ff79_ffff_0000;
pub const KSTACK_START: usize  = 0xffff_ff79_fffe_0000;
pub const KSTACK_END: usize    = 0xffff_ff78_0000_0000;
pub const GUARD_PAGE: usize    = 0xffff_ff77_ffff_0000;
pub const STACK_START: usize   = 0xffff_ff77_fffe_0000;
pub const STACK_END: usize     = 0xffff_ff00_0001_0000;
pub const DATA_END: usize      = 0x0000_8fff_fff0_0000;
//pub const DATA_START: usize    = 0x0000_0200_0000_0000;
//pub const BSS_END: usize       = 0x0000_01ff_ffff_0000;
//pub const BSS_START: usize     = 0x0000_01f0_0000_0000;
pub const DATA_START: usize    = 0x0000_01f0_0000_0000;
pub const CODE_END: usize      = 0x0000_01ef_ffff_0000;
pub const CODE_START: usize    = 0x0000_0000_0101_0000;
pub const TEMP_MAP: usize      = 0x0000_0000_0081_0000;
pub const KERNEL_END: usize    = 0x0000_0000_0080_0000;
pub const KERNEL_START: usize  = 0x0000_0000_0001_0000;
pub const ZERO_ADDR: usize     = 0x0000_0000_0000_0000;
pub const UGUARD_PAGE: usize   = 0xffff_ff00_0000_0000;


#[repr(align(4096))]
#[derive(Copy, Clone)]
pub struct StaticPage([u8; PAGE_SIZE]);

impl StaticPage {
  pub const fn new() -> StaticPage {
    StaticPage{0:[0; PAGE_SIZE]}
  }
  pub fn to_physaddr(&mut self) -> PhysAddr {
    PhysAddr::new((self as *mut StaticPage) as u64).
    expect("static pages must be allocated on non-null pointer")
  }
}

use self::pagelist::PagePool;
use self::pagelist::pagelist_ng::{PageMap, PageMapWrapper};
use self::pagelist::{PagePoolReleaseError, PagePoolAllocationError, PagePoolAppendError};

#[repr(C)]
#[repr(align(4096))]
pub struct PageManager {
  pagepool: Option<PageMapWrapper>,
}

// Memory allocated for bootstrapping
static mut BOOT_PAGES: [StaticPage; BOOT_MEMORY_PAGES as usize] = 
  [crate::vmem::StaticPage::new(); BOOT_MEMORY_PAGES as usize];

#[derive(Debug)]
pub enum InitError {
  NoneError(NoneError),
  AllocError(PagePoolAllocationError),
  Infallible(core::convert::Infallible)
}

impl From<PagePoolAllocationError> for InitError {
  fn from(p: PagePoolAllocationError) -> InitError { InitError::AllocError(p) }
}

impl From<core::convert::Infallible> for InitError {
  fn from(p: core::convert::Infallible) -> InitError { InitError::Infallible(p) }
}

impl From<NoneError> for InitError {
  fn from(p: NoneError) -> InitError { InitError::NoneError(p) }
}

impl From<()> for InitError {
  fn from(p: ()) -> InitError { InitError::NoneError(NoneError) }
}

impl<'a> PageManager {
  pub const fn new() -> PageManager { PageManager { pagepool: None, } }

  pub fn init(&mut self) -> Result<(), InitError> {
    trace!("allocating pagepool");
    {
      self.pagepool = Some(PageMap::new_no_alloc(
        PhysAddr::try_from(self.get_boot_base())?, BOOT_MEMORY_PAGES as u16
      )?.try_into()?);
    }
    trace!("pagepool allocated");
    Ok(())
  }
  fn pagepool(&self) -> &dyn PagePool {
    self.pagepool.as_ref().unwrap()
  }
  fn pagepool_mut(&mut self) -> &mut dyn PagePool {
    self.pagepool.as_mut().unwrap()
  }
  fn get_boot_base(&self) -> PhysAddr {
    unsafe{
      PhysAddr::new(
        (&mut BOOT_PAGES[0].0[0] as *mut u8) as u64).
        expect("must have boot base")
    }
  }

  fn print_free_mem(&self) {
    let pages = self.free_memory();
    let mem = pages * 4096;
    trace!("Free memory now {} KiB, {} MiB, {} Pages",
      mem / 1024,
      mem / 1024 / 1024,
      pages
    );
  }

  pub unsafe fn add_memory(&mut self, start: PhysAddr, num_pages: usize) -> Result<(), PagePoolAppendError> {
    self.print_free_mem();
    trace!("Adding MMAPE {}+{} to pool", start, num_pages);
    self.pagepool_mut().add_memory(start, num_pages)?;
    self.print_free_mem();
    Ok(())
  }
  pub fn free_memory(&self) -> usize {
    self.pagepool().count_free()
  }
  fn from_objpage(page: &mut ObjectPage<'a>) -> *mut u8 {
    ((page as *mut ObjectPage) as usize) as *mut u8
  }
  fn to_objpage(ptr: *mut u8) -> &'a mut ObjectPage<'a> {
    unsafe { &mut *((ptr as usize) as *mut ObjectPage) }
  }
  pub unsafe fn alloc_page(&mut self) -> Result<PhysAddr, PagePoolAllocationError> {
    self.pagepool_mut().allocate()
  }
  pub unsafe fn free_page(&mut self, pa: PhysAddr) -> Result<(), PagePoolReleaseError> {
    self.pagepool_mut().release(pa)
  }
}

impl<'a> ::slabmalloc::PageProvider<'a> for PageManager {
  fn allocate_page(&mut self) -> Option<&'a mut ObjectPage<'a>> {
    trace!("Allocating Page...");
    if let Ok(page) = self.pagepool_mut().allocate() {
      return Some(PageManager::to_objpage(page.as_u64() as *mut u8));
    }
    None
  }
  fn release_page(&mut self, page: &mut ObjectPage<'a>) {
    trace!("Releasing Page {:?}", page);
    let addr = PageManager::from_objpage(page);
    let addr = addr.try_into().expect("release page must be valid address");
    {
      //TODO: make sure this is safe, page may be unmapped!
      trace!("Clearing page data...");
      let page_raw = (page as *mut _) as *mut [u8; PAGE_SIZE];
      for x in 0..PAGE_SIZE-1 {
        unsafe { (*page_raw)[x] = 0x00; }
      }
    }
    self.pagepool_mut().release(addr).expect("could not release page for allocator");
  }
}

unsafe impl Send for PageManager {}
unsafe impl Sync for PageManager {}