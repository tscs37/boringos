pub mod pagelist;
pub mod pagetable;
pub mod mapper;
pub mod faulth;

use core::convert::TryInto;
use core::option::NoneError;
use atomic::Atomic;
use crate::common::{PhysAddr, PhysFrame};
use crate::*;

pub const PAGE_SIZE: usize = 4096;

const BOOT_MEMORY_PAGES: u16 = 16;

pub const KSTACK_START: usize  = 0xffff_ff80_0000_0000;
pub const KSTACK_END: usize    = 0xffff_ff70_0000_0000;
pub const KSTACK_GUARD: usize  = 0xffff_ff70_0000_0000;
pub const KHEAP_END: usize     = 0xffff_ff69_0000_0000;
pub const KHEAP_ALLOC: usize   = 0xffff_ff61_1000_0000;
pub const KHEAP_START: usize   = 0xffff_ff61_0000_0000;
pub const GUARD_PAGE: usize    = 0xffff_ff60_0000_0000;
pub const STACK_START: usize   = 0xffff_ff5f_ffff_0000;
pub const STACK_END: usize     = 0xffff_ff00_0001_0000;
pub const DATA_END: usize      = 0x0000_8fff_fff0_0000;
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
    PhysAddr::try_new((self as *mut StaticPage) as u64).
    expect("static pages must be allocated on non-null pointer")
  }
}

use self::pagelist::PagePool;
use self::pagelist::pagelist_ng::{PageMap, PageMapWrapper};
use self::pagelist::{PagePoolReleaseError, PagePoolAllocationError, PagePoolAppendError};

#[repr(C)]
#[repr(align(4096))]
pub struct PageManager {
  pagepool: Atomic<Option<PageMapWrapper>>,
}

// Memory allocated for bootstrapping
static mut BOOT_PAGES: [StaticPage; BOOT_MEMORY_PAGES as usize] = 
  [crate::vmem::StaticPage::new(); BOOT_MEMORY_PAGES as usize];

#[derive(Debug)]
pub enum InitError {
  NoneError(NoneError),
  AllocError(PagePoolAllocationError),
  Infallible(core::convert::Infallible),
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

impl PageManager {
  pub const fn new() -> PageManager { PageManager { pagepool: Atomic::new(None), } }

  pub unsafe fn init(&self, physical_memory_offset: VirtAddr) -> Result<(), InitError> {
    self.internal_init(physical_memory_offset)
  }

  pub unsafe fn overwrite_pagepool(&self, pm: PageMapWrapper) {
    self.pagepool.store(Some(pm), atomic::Ordering::SeqCst);
  }

  pub unsafe fn erase_pagepool(&self) {
    self.pagepool.store(None, atomic::Ordering::SeqCst);
  }

  fn internal_init(&self, physical_memory_offset: VirtAddr) -> Result<(), InitError> {
    trace!("allocating pagepool");
    {
      self.pagepool.store(Some(PageMap::new_no_alloc(
        self.get_boot_base(), BOOT_MEMORY_PAGES as u16
      )?.try_into()?), atomic::Ordering::SeqCst);
    }
    trace!("pagepool allocated");

    use x86_64::structures::paging::Page;
    let start = KHEAP_START;
    let size = KHEAP_END - KHEAP_START;
      
    debug!("mapping first heap page");
    {
      let page = Page::containing_address(VirtAddr::new(KHEAP_START.try_into().unwrap()));
      let frame = self.pagepool().allocate().expect("require page for initial heap");
      debug!("working on page {:#018x}", page.start_address().as_u64());
      let flags = vmem::mapper::MapType::Data.flags();
      debug!("mapping with flags {:?}", flags);
      pagetable::get_pagemap_mut(|mapper| {
        debug!("got page frame: {:#018x}", frame.start_address().as_u64());
        use x86_64::structures::paging::mapper::Mapper;
        unsafe{mapper.map_to(page, frame, flags, &mut self.pagepool
          .load(atomic::Ordering::Relaxed)
          .unwrap()).expect("failed to map").flush()};
      });
    }
    trace!("init pagetable");
    unsafe{crate::vmem::pagetable::init(physical_memory_offset)};
    trace!("pagetable init ok");
    Ok(())
  }

  pub fn pagepool(&self) -> PageMapWrapper {
    self.pagepool.load(atomic::Ordering::Relaxed).expect("pagepool not installed but expected")
  }

  fn get_boot_base(&self) -> PhysAddr {
    unsafe{
      PhysAddr::new((&mut BOOT_PAGES[0].0[0] as *mut u8) as u64)
    }
  }

  pub fn print_free_mem(&self) {
    let pages = self.free_memory();
    let mem = pages * 4096;
    trace!("Free memory now {} KiB, {} MiB, {} Pages",
      mem / 1024,
      mem / 1024 / 1024,
      pages
    );
  }

  pub fn print_used_mem(&self) {
    let pages = self.used_memory();
    let mem = pages * 4096;
    trace!("Used memory now {} KiB, {} MiB, {} Pages",
      mem / 1024,
      mem / 1024 / 1024,
      pages
    );
  } 

  pub fn print_total_mem(&self) {
    let pages = self.total_memory();
    let mem = pages * 4096;
    trace!("Total memory now {} KiB, {} MiB, {} Pages",
      mem / 1024,
      mem / 1024 / 1024,
      pages
    );
  } 

  pub fn print_mem_summary(&self) {
    self.print_total_mem();
    self.print_used_mem();
    self.print_free_mem();
  }

  pub fn pagemap_layout() -> alloc::alloc::Layout {
    alloc::alloc::Layout::new::<PageMap>()
  } 

  // Add memory to the pagepool, requires a PageMap-sized allocation to be passed
  // Returns number of pages added to the pool, loop until this number is 0
  pub unsafe fn add_memory(&self, alloc: *mut PageMap, start: PhysAddr, num_pages: u64) -> Result<u64, PagePoolAppendError> {
    Ok(self.pagepool().add_memory(alloc, start, num_pages)?)
  }
  pub fn free_memory(&self) -> usize {
    self.pagepool().count_free()
  }
  pub fn total_memory(&self) -> usize {
    self.pagepool().count_all()
  } 
  pub fn used_memory(&self) -> usize {
    self.pagepool().count_used()
  } 
  pub unsafe fn alloc_page(&self) -> Result<PhysAddr, PagePoolAllocationError> {
    self.pagepool().allocate().map(|x| x.start_address())
  }
  pub unsafe fn free_page(&self, pa: PhysAddr) -> Result<(), PagePoolReleaseError> {
    self.pagepool().release(PhysFrame::containing_address(pa))
  }
}

unsafe impl Send for PageManager {}
unsafe impl Sync for PageManager {}