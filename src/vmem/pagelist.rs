

const PagesPerBlock: usize = 512;
use vmem::PageSize;
use ::core::ptr::NonNull;

// we ignore address 0
pub struct PhysAddr(NonNull<u8>);

impl PhysAddr {
  pub fn as_u64(&self) -> u64 {
    self.as_mut8() as u64
  }
  pub fn as_mut8(&self) -> *mut u8 {
    self.0.as_ptr()
  }
  pub fn as_ptr<T>(&self) -> *mut T {
    self.0.as_ptr() as *mut T
  }
  pub fn as_physaddr(&self) -> ::x86_64::PhysAddr {
    ::x86_64::PhysAddr::new(self.0.as_ptr() as u64)
  }
}

impl ::core::cmp::PartialEq for PhysAddr {
  fn eq(&self, rhs: &PhysAddr) -> bool {
    self.as_u64() == rhs.as_u64()
  }
}

#[repr(C)]
#[repr(align(4096))]
pub struct PageList {
  pub pages: [Option<PhysAddr>; PagesPerBlock],
  pub used: [bool; PagesPerBlock],
  pub next: PageListLink,
  pub prev: PageListLink,
  pub lowest: PhysAddr,
  pub highest: PhysAddr,
}

assert_eq_size!(check_page_list_size; PageList, [u8;4096]);

#[repr(C)]
pub struct PageRange {
  pub start: PhysAddr,
  pub pages: usize,
  pub next: PageListLink,
  pub prev: PageListLink,
}

pub enum PageListLink {
  None,
  PageRangeEntry(NonNull<PageRange>), // Start and number of 4096 Pages
  PageListEntry(NonNull<PageList>),
}

impl PageList {
  pub fn new(p: PhysAddr) -> *mut PageList {
    zero_page(p);
    p.as_u64() as *mut PageList
  }
  pub fn grab_free(&mut self) -> Option<PhysAddr> {
    for x in 0..PagesPerBlock {
      if self.pages[x].is_some() {
        if !self.used[x] {
          self.used[x] = true;
          return Some(self.pages[x].expect("tested page presence above, must exist here"));
        }
      }
    }
    match self.next {
      PageListLink::PageListEntry(next_pl) => {
        unsafe { (*next_pl.as_ptr()).grab_free() }
      },
      PageListLink::PageRangeEntry(pr) => panic!("not implemented"),
      PageListLink::None => None,
    }
  }
  fn grab_two(&mut self) -> Option<(PhysAddr, PhysAddr)> {
   if let Some(pa) = self.grab_free() {
     if let Some(pa2) = self.grab_free() {
       return Some((pa, pa2))
     } else {
       self.release(pa);
     }
   }
   None
  }
  pub fn release(&mut self, p: PhysAddr) {
    if !(p.as_u64() < self.lowest.as_u64() || p.as_u64() > self.highest.as_u64()) {
      for x in 0..PagesPerBlock {
        match self.pages[x] {
          Some(page) => {
            if page == p {
              self.used[x] = false;
              zero_page(p);
              return;
            }
          }
          None => (),
        }
      }
    }
    match self.next {
      PageListLink::PageListEntry(next_pl) => {
        unsafe { (*next_pl.as_ptr()).release(p) }
      },
      PageListLink::PageRangeEntry(pr) => panic!("not implemented"),
      PageListLink::None => panic!("could not release address"),
    }
  }
  pub fn append_range(&mut self, base: PhysAddr, size: usize) {
    panic!("TODO")
  }
  // convert_range will create a PageList and a PageRange out of a given PageListLink, which must
  // be a PageRangeEntry. The PageRange will have at most "PagesPerBlock" in pages consumed and
  // the adjusted values will be returned.
  fn convert_range(&mut self, pll: PageListLink) -> (PageRange, *mut PageList) {
    if let PageListLink::PageRangeEntry(pr) = pll {
      if let Some(pla) = self.grab_free() {
        let pl = PageList::new(pla);
        let pla_e = pla as *mut PageRange;
        // Consume all pages that fit into the block
        (PageListLink::PageRangeEntry(PageRange{
          start: pa.start + PageSize * PagesPerBlock, 
          size: pa.size - PagesPerBlock,
          next: pa.next,
          prev: pa.previous,
        }), pla.as_u64() as *mut PageList)
      } else {
        let new_pa = pa + PageSize * (PagesPerBlock + 1);
        let new_size = size - (PagesPerBlock + 1); // Adjust range
        zero_page(pa);
        (PageListLink::PageRange(new_pa, new_size), pa.as_u64() as *mut PageList)
      }
    } else {
      panic!("attempted to convert non-pagerange to pagelist entry");
    }
  }
}

fn zero_page(page: PhysAddr) {
  use ::vmem::PageSize;
  let page_raw = page.as_u64() as *mut [u8; PageSize];
  for x in 0..PageSize {
    unsafe { (*page_raw)[x] = 0x00 };
  }
}