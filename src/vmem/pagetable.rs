use ::vmem::PageManager;
use core::ops::{Index, IndexMut};
use core::marker::PhantomData;
use core::ptr::NonNull;
use ::vmem::PAGE_SIZE;
use ::vmem::PhysAddr;

const ENTRY_COUNT: usize = 512;
const LO_ADDR_SPACE: usize = 0x0000_8000_0000_0000;
const HI_ADDR_SPACE: usize = 0xffff_8000_0000_0000;
const PAGE_ADDR_FILTER: u64 = 0x000fffff_fffff000;
pub const P4: *mut Table<Level4> = 0xffffffff_fffff000 as *mut _;

pub type PagePhysAddr = usize;
pub type PageVirtAddr = usize;

pub trait TableLevel {}
pub trait HierarchicalLevel: TableLevel {
  type NextLevel: TableLevel;
}
pub enum Level4 {}
pub enum Level3 {}
pub enum Level2 {}
pub enum Level1 {}
impl TableLevel for Level4 {}
impl TableLevel for Level3 {}
impl TableLevel for Level2 {}
impl TableLevel for Level1 {}
impl HierarchicalLevel for Level4 {
  type NextLevel = Level3;
}
impl HierarchicalLevel for Level3 {
  type NextLevel = Level2;
}
impl HierarchicalLevel for Level2 {
  type NextLevel = Level1;
}

pub struct ActivePageTable {
  p4: NonNull<Table<Level4>>,
}

pub struct Page {
  number: usize,
}

pub struct Entry(u64);

impl Page {
  fn start_address(&self) -> usize {
    self.number * PAGE_SIZE
  }
  pub fn containing_address(vaddr: PageVirtAddr) -> Page {
    assert!(
      vaddr < LO_ADDR_SPACE ||
      vaddr > HI_ADDR_SPACE
    );
    Page { number: vaddr / PAGE_SIZE }
  }
  fn p4_index(&self) -> usize {
    (self.number >> 27) & 0o777
  }
  fn p3_index(&self) -> usize {
    (self.number >> 18) & 0o777
  }
  fn p2_index(&self) -> usize {
    (self.number >> 9) & 0o777
  }
  fn p1_index(&self) -> usize {
    (self.number >> 0) & 0o777
  }
}

impl ActivePageTable {
  pub unsafe fn new() -> ActivePageTable {
    ActivePageTable {
      p4: NonNull::new_unchecked(P4),
    }
  }
  fn p4(&self) -> &Table<Level4> {
    unsafe { self.p4.as_ref() }
  }
  fn p4_mut(&mut self) -> &mut Table<Level4> {
    unsafe { self.p4.as_mut() }
  }

  pub fn translate(&self, vaddr: PageVirtAddr) -> Option<PagePhysAddr> {
    let offset = vaddr % PAGE_SIZE;
    self.translate_page(Page::containing_address(vaddr))
      .map(|frame| 
        frame.as_usize() * PAGE_SIZE + offset)
  }

  fn translate_page(&self, page: Page) -> Option<PhysAddr> {
    let p3 = self.p4().next_table(page.p4_index());
    let huge_page = || {
      p3.and_then(|p3|{
        let p3_entry = &p3[page.p3_index()];
        if let Some(start_frame) = p3_entry.real_addr() {
          if p3_entry.flags().contains(EntryFlags::HUGE_PAGE) {
            // address 1GiB aligned
            assert!(start_frame.as_usize() % (ENTRY_COUNT * ENTRY_COUNT) == 0);
            return PhysAddr::new_usize(
              start_frame.as_usize() + page.p2_index() * ENTRY_COUNT +
                page.p1_index()
            );
          }
        }
        if let Some(p2) = p3.next_table(page.p3_index()) {
          let p2_entry = &p2[page.p2_index()];
          if let Some(start_frame) = p2_entry.real_addr() {
            if p2_entry.flags().contains(EntryFlags::HUGE_PAGE) {
              assert!(start_frame.as_usize() % ENTRY_COUNT == 0);
              return PhysAddr::new_usize(
                start_frame.as_usize() + page.p1_index()
              );
            }
          }
        }
        None
      })
    };

    p3.and_then(|p3| p3.next_table(page.p3_index()))
      .and_then(|p2| p2.next_table(page.p2_index()))
      .and_then(|p1| p1[page.p1_index()].real_addr())
      .or_else(huge_page)
  }

  pub fn map_to(&mut self, page: Page, target: PhysAddr, flags: EntryFlags,
    pm: &mut PageManager) {
      let mut p3 = self.p4_mut().next_table_create(page.p4_index(), pm);
      let mut p2 = p3.next_table_create(page.p3_index(), pm);
      let mut p1 = p2.next_table_create(page.p2_index(), pm);
      assert!(p1[page.p1_index()].is_unused());
      p1[page.p1_index()].set_addr(target, 
        flags | EntryFlags::PRESENT);
    }
  pub fn map(&mut self, page: Page, flags: EntryFlags,
    pm: &mut PageManager) {
      let frame = unsafe { pm.alloc_page() }.expect("out of memory");
      self.map_to(page, frame, flags, pm)
    }

  pub fn identity_map(&mut self, page: Page, flags: EntryFlags,
    pm: &mut PageManager) {
      let frame = PhysAddr::new_usize(page.start_address())
        .expect("cannot identity map 0x0");
      self.map_to(page, frame, flags, pm)
    }

  pub fn unmap(&mut self, page: Page, flags: EntryFlags,
    pm: &mut PageManager) {
      let p1 = self.p4_mut()
        .next_table_mut(page.p4_index())
        .and_then(|p3| p3.next_table_mut(page.p3_index()))
        .and_then(|p2| p2.next_table_mut(page.p2_index()))
        .expect("mapping code does not support huge pages");
      let frame = p1[page.p1_index()].real_addr().unwrap();
      p1[page.p1_index()].set_unused();
      use x86_64::instructions::tlb;
      use x86_64::VirtAddr;
      tlb::flush(VirtAddr::new(page.start_address() as u64));
      unsafe { pm.free_page(frame) }
    }
}
impl Entry {
  pub fn is_unused(&self) -> bool {
    self.0 == 0
  }
  pub fn set_unused(&mut self) {
    self.0 = 0
  }
  pub fn flags(&self) -> EntryFlags {
    EntryFlags::from_bits_truncate(self.0)
  }
  pub fn real_addr(&self) -> Option<PhysAddr> {
    if self.flags().contains(EntryFlags::PRESENT) {
      let addrnn = NonNull::new(
        (self.0 & PAGE_ADDR_FILTER) as *mut u8
      );
      match addrnn {
        Some(addr) => {
          Some(PhysAddr::from(addr))
        }
        None => None,
      }
    } else {
      None
    }
  }
  pub fn set_addr(&mut self, pa: PhysAddr, flags: EntryFlags) {
    assert!(pa.as_u64() & !PAGE_ADDR_FILTER == 0);
    self.0 = pa.as_u64() | flags.bits();
  }
}

bitflags! {
  pub struct EntryFlags: u64 {
    const PRESENT =         1 << 0;
    const WRITABLE =        1 << 1;
    const USER_ACCESSIBLE = 1 << 2;
    const WRITE_THROUGH =   1 << 3;
    const NO_CACHE =        1 << 4;
    const ACCESSED =        1 << 5;
    const DIRTY =           1 << 6;
    const HUGE_PAGE =       1 << 7;
    const GLOBAL =          1 << 8;
    const OS_EXTERNAL =     1 << 10;
    const NO_EXECUTE =      1 << 63;
  }
}

pub struct Table<L: TableLevel> {
  entries: [Entry; ENTRY_COUNT],
  level: PhantomData<L>,
}

impl<L> Table<L> where L: TableLevel {
  pub fn zero(&mut self) {
    for entry in self.entries.iter_mut() {
      entry.set_unused();
    }
  }
}

impl<L> Table<L> where L: HierarchicalLevel {
  fn next_table_address(&self, index: usize) -> Option<usize> {
    let entry_flags = self[index].flags();
    if entry_flags.contains(EntryFlags::PRESENT) && 
      !entry_flags.contains(EntryFlags::HUGE_PAGE) {
        let table_address = self as *const _ as usize;
        Some((table_address << 9) | (index << 12))
    } else {
      None
    }
  }
  pub fn next_table(&self, index: usize) -> Option<&Table<L::NextLevel>> {
    self.next_table_address(index)
      .map(|address| unsafe { &*(address as *const _) })
  }
  pub fn next_table_mut(&self, index:usize) -> Option<&mut Table<L::NextLevel>> {
    self.next_table_address(index)
      .map(|address| unsafe { &mut *(address as *mut _) })
  }
  pub fn next_table_create(&mut self, index: usize, pm: &mut PageManager)
    -> &mut Table<L::NextLevel>
    {
      if self.next_table(index).is_none() {
        let frame = unsafe { pm.alloc_page() }
          .expect("no free memory available");
        self.entries[index].set_addr(frame, 
          EntryFlags::PRESENT | EntryFlags::WRITABLE);
        self.next_table_mut(index).unwrap().zero();
      }
      self.next_table_mut(index).unwrap()
    }
}

impl<L> Index<usize> for Table<L> where L: TableLevel {
  type Output = Entry;

  fn index(&self, index: usize) -> &Entry {
    &self.entries[index]
  }
}

impl<L> IndexMut<usize> for Table<L> where L: TableLevel {
  fn index_mut(&mut self, index: usize) -> &mut Entry {
    &mut self.entries[index]
  }
}
