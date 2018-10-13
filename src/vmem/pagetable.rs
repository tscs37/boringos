use core::ops::{Index, IndexMut};
use core::marker::PhantomData;
use core::ptr::NonNull;
use ::vmem::PAGE_SIZE;
use ::vmem::PhysAddr;

const ENTRY_COUNT: usize = 512;
const PAGE_ADDR_FILTER: u64 = 0x000fffff_fffff000;
pub const P4: *mut Table<Level4> = 0xffffffff_fffff000 as *mut _;

struct PagePhysAddr(usize);
struct PageVirtAddr(usize);

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

pub struct Page {
  number: usize,
}

pub struct Entry(u64);

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