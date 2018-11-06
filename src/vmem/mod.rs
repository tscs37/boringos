pub mod pagelist;
pub mod pagetable;
pub mod mapper;

use ::slabmalloc::ObjectPage;
use ::vmem::pagelist::PageListLink;

pub const PAGE_SIZE: usize = 4096;

pub const PAGE_TABLE_LO: usize = 0xffff_ff80_0000_0000;
pub const KSTACK_GUARD: usize  = 0xffff_ff79_ffff_0000;
pub const KSTACK_START: usize  = 0xffff_ff79_fffe_0000;
pub const KSTACK_END: usize    = 0xffff_ff78_0000_0000;
pub const GUARD_PAGE: usize    = 0xffff_ff77_ffff_0000;
pub const STACK_START: usize   = 0xffff_ff77_fffe_0000;
pub const STACK_END: usize     = 0xffff_ff00_0001_0000;
pub const UGUARD_PAGE: usize   = 0xffff_ff00_0000_0000;

pub use ::vmem::pagelist::PhysAddr;
pub use ::vmem::pagetable::PAGE_ADDR_FILTER;

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

#[repr(C)]
#[repr(align(4096))]
pub struct PageManager {
  pub first_page_mem: StaticPage,
  pub first_range_mem: StaticPage,
  pub boot_pages: [StaticPage; ::BOOT_MEMORY_PAGES],
  pub use_boot_memory: bool,
  // list of 4k pages
  //pub pages: Option<Arc<Uns
  pub pages: pagelist::PageListLink,
}

impl<'a> PageManager {
  pub unsafe fn init_page_store(&mut self) {
    debug!("Initializing Page Store...");
    match self.pages {
      PageListLink::None => {
        self.pages = PageListLink::PageListEntry(
          pagelist::PageList::new(self.first_page_mem.to_physaddr()));
        use ::vmem::pagelist::PageRange;
        let mut pr = PageListLink::PageRangeEntry(
          PageRange::new(self.first_range_mem.to_physaddr(),
            self.get_boot_base(),
            self.boot_pages.len()
        ));
        self.pages.set_next(pr);
        pr.set_prev(self.pages);
      }
      _ => panic!("attempted to double init page store"),
    }
    debug!("Page Store initialized with free memory: {} KiB", self.free_memory() / 1024);
  }
  fn get_boot_base(&mut self) -> PhysAddr {
    PhysAddr::new(
      (&mut self.boot_pages[0].0[0] as *mut u8) as u64).
      expect("must have boot base")
  }

  pub unsafe fn add_memory(&mut self, start: PhysAddr, num_pages: usize) {
    match self.pages {
      PageListLink::PageListEntry(_) => {
        self.pages.append_range(start, num_pages);
      }
      _ => { panic!("attempted to add memory but couldn't get pages")}
    }
    {
      let pages = self.pages.free_pages();
      let mem = pages * 4096;
      debug!("Free memory now {} KiB, {} MiB, {} Pages",
        mem / 1024,
        mem / 1024 / 1024,
        pages
      )
    }
  }
  pub fn free_memory(&self) -> usize {
    match self.pages {
      PageListLink::PageListEntry(_) => {
        return self.pages.free_pages() * 4096;
      }
      _ => { panic!("attempted to count free memory but couldn't get pages")}
    }
  }
  fn from_objpage(page: &mut ObjectPage<'a>) -> *mut u8 {
    ((page as *mut ObjectPage) as usize) as *mut u8
  }
  fn to_objpage(ptr: *mut u8) -> &'a mut ObjectPage<'a> {
    unsafe { &mut *((ptr as usize) as *mut ObjectPage) }
  }
  pub unsafe fn alloc_page(&mut self) -> Option<PhysAddr> {
    self.pages.grab_free()
  }
  pub unsafe fn free_page(&mut self, pa: PhysAddr) {
    self.pages.release(pa)
  }
}

impl<'a> ::slabmalloc::PageProvider<'a> for PageManager {
  fn allocate_page(&mut self) -> Option<&'a mut ObjectPage<'a>> {
    //debug!("Allocating Page...");
    if let Some(page) = self.pages.grab_free() {
      return Some(PageManager::to_objpage(page.as_u64() as *mut u8));
    }
    None
  }
  fn release_page(&mut self, page: &mut ObjectPage<'a>) {
    //debug!("Releasing Page {:?}", page);
    {
      //debug!("Clearing page data...");
      let page_raw = (page as *mut _) as *mut [u8; PAGE_SIZE];
      for x in 0..PAGE_SIZE-1 {
        unsafe { (*page_raw)[x] = 0x00; }
      }
    }
    let addr = PageManager::from_objpage(page);
    match self.pages {
      PageListLink::PageListEntry(_) => {
        self.pages.release(unsafe { PhysAddr::new_unchecked(addr as u64) });
      }
      _ => { panic!("tried to dealloc non-boot page without page struct") }
    }
  }
}

unsafe impl Send for PageManager {}
unsafe impl Sync for PageManager {}