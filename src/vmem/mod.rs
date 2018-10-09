pub mod pagelist;

use ::slabmalloc::ObjectPage;
use ::vmem::pagelist::PageListLink;

pub const PageSize: usize = 4096;

//use x86_64::PhysAddr;
use ::vmem::pagelist::PhysAddr;

#[repr(C)]
#[repr(align(4096))]
pub struct PageManager {
  pub boot_pages: [[u8; PageSize]; ::BOOT_MEMORY_PAGES],
  pub boot_used: [bool; ::BOOT_MEMORY_PAGES],
  pub use_boot_memory: bool,
  // list of 4k pages
  //pub pages: Option<Arc<UnsafeCell<BinaryHeap<PhysAddr>>>>,
  pub pages: pagelist::PageListLink,
}

impl<'a> PageManager {
  pub unsafe fn init_page_store(&mut self) {
    debug!("Initializing Page Store...");
    match self.pages {
      PageListLink::None => {
        // We need to allocate here so we need to force open the lock for a moment
        unsafe { ::PAGER.force_unlock(); }
        match self.get_boot_page() {
          Some(page) => {
            self.pages = PageListLink::PageListEntry(pagelist::PageList::new(page));
          }
          None => panic!("could not get page for first pagelist"),
        }
      }
      _ => panic!("attempted to double init page store"),
    }
    debug!("Done, returning with new page store");
  }
  fn get_boot_page(&mut self) -> Option<PhysAddr> {
    for x in 0..self.boot_used.len() {
      if !self.boot_used[x] {
        debug!("Allocating BootPage {}", x);
        self.boot_used[x] = true;
        return PhysAddr::new((&mut self.boot_pages[x][0] as *mut u8) as u64);
      }
    }
    None
  }
  fn free_boot_page(&mut self, pa: PhysAddr) {
    let base = self.get_boot_base();
    let pos = (pa.as_u64() - base.as_u64()) / PageSize as u64;
    debug!("Freeing BootPage {}", pos);
    self.boot_used[pos as usize] = false;
  }

  fn get_boot_base(&self) -> PhysAddr {
    PhysAddr::new((self.boot_pages[0][0] as *mut u8) as u64).expect("must have boot base")
  }
  fn get_boot_top(&self) -> PhysAddr {
    PhysAddr::new((self.boot_pages[self.boot_pages.len() - 1][0] as *mut u8) as u64).expect("must have boot top")
  }

  pub unsafe fn add_memory(&self, start: PhysAddr, num_pages: usize) {
    debug!("enter: add_memory");
    //TODO: use self.pages.insert/append
    match self.pages {
      PageListLink::PageListEntry(_) => {
        self.pages.append_range(start, num_pages);
      }
      _ => { panic!("attempted to add memory but couldn't get pages")}
    }
    debug!("leave: add_memory");
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
}

impl<'a> ::slabmalloc::PageProvider<'a> for PageManager {
  fn allocate_page(&mut self) -> Option<&'a mut ObjectPage<'a>> {
    debug!("Allocating Page...");
    if self.use_boot_memory || true {
      if let Some(page) = self.get_boot_page() {
        debug!("Got a page from boot: {}", page);
        return Some(PageManager::to_objpage(page.as_u64() as *mut u8));
      }
      debug!("Boot Memory exhausted, using real memory...");
    }
    None
    //TODO: use self.pages.grab_free()
    /*match self.pages {
      Some(ref pages) => {
        let mut pgs = unsafe { &mut *(pages.clone()).get() };
        match pgs.pop() {
          Some(page) => {
            debug!("Got a page from memory: {:#016x}", page);
            Some(PageManager::to_objpage(page.as_u64() as *mut u8))
          }
          None => { None }
        }
      }
      None => { None }
    }*/
  }
  fn release_page(&mut self, page: &mut ObjectPage<'a>) {
    debug!("Releasing Page {:?}", page);
    {
      debug!("Clearing page data...");
      let page_raw = (page as *mut _) as *mut [u8; PageSize];
      for x in 0..PageSize-1 {
        unsafe { (*page_raw)[x] = 0x00; }
      }
    }
    let addr = PageManager::from_objpage(page);
    if (addr as u64) < self.get_boot_top().as_u64() {
      if addr as u64 > self.get_boot_base().as_u64() {
        self.free_boot_page(PhysAddr::new_unchecked(addr as u64));
      }
    }
    match self.pages {
      PageListLink::PageListEntry(_) => {
        self.pages.release(PhysAddr::new_unchecked(addr as u64));
      }
      _ => { panic!("tried to dealloc non-boot page without page struct") }
    }
  }
}

unsafe impl Send for PageManager {}
unsafe impl Sync for PageManager {}