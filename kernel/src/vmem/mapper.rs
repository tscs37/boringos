use crate::process_manager::TaskHandle;
use crate::vmem::pagetable::Page;
use crate::vmem::PhysAddr;
use crate::vmem::PAGE_SIZE;
use crate::*;
use x86_64::structures::paging::PageTableFlags;
use x86_64::structures::paging::Size4KiB;
use x86_64::structures::paging::mapper::MapperAllSizes;
use x86_64::structures::paging::mapper::Mapper;
use vmem::pagetable::{get_pagemap, get_pagemap_mut, get_pagetable};

#[derive(PartialEq, Debug)]
pub enum MapType {
  Stack,               // Stack Page, No Execute
  Data,                // Data Page, No Execute
  UnsafeCode,          // Code Page, Writable
  Code,                // Code Page, No Write
  ReadOnly,            // Data Page, No Write
  Managed(TaskHandle), // Memory available via other process
  ShMem(TaskHandle),   // Memory shared to other process
  Guard,               // No Execute, No Read+Write
  Zero,                // Map No Execute, No RW Page, share page
  Empty,               // No Flags
}

impl MapType {
  pub fn flags(&self) -> PageTableFlags {
    let base_flags = match self {
      MapType::Stack => PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE,
      MapType::Data => PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE,
      MapType::UnsafeCode => PageTableFlags::WRITABLE,
      MapType::Code => PageTableFlags::empty() | PageTableFlags::WRITABLE,
      MapType::ReadOnly => PageTableFlags::NO_EXECUTE,
      MapType::Managed(_) => PageTableFlags::NO_EXECUTE,
      MapType::ShMem(_) => PageTableFlags::NO_EXECUTE,
      MapType::Guard => PageTableFlags::NO_EXECUTE,
      MapType::Zero => PageTableFlags::NO_EXECUTE,
      MapType::Empty => return PageTableFlags::empty(),
    };
    base_flags | PageTableFlags::PRESENT
  }
}

impl Default for MapType {
  fn default() -> MapType {
    MapType::Empty
  }
}

use x86_64::structures::paging::PhysFrame;

pub fn map_new(base_addr: VirtAddr, mt: MapType) -> PhysAddr {
  trace!("mapping new page to {:?} ({:?})", base_addr, mt);
  let pm = pager();
  let flags = mt.flags();
  let frame = unsafe{ pm.alloc_page().expect("map new failed") };
  let page: Page<Size4KiB> = Page::containing_address(base_addr);
  let pagepool = &mut pm.pagepool().clone();
  trace!("putting new page into pagetable");
  get_pagemap_mut(|apt| {
    let res = unsafe { apt.map_to(page, PhysFrame::containing_address(frame), 
      flags, pagepool) };
    let res = res.unwrap();
    res.flush();
    frame
  })
}

pub fn dump_pagetable() {
  use x86_64::structures::paging::PageTable;
  get_pagetable(|apt: &PageTable| {
    for (idx, pte) in apt.iter().enumerate() {
      if idx == 1 { continue }
      if pte.flags().contains(PageTableFlags::PRESENT) {
        trace!("{}: {:?}", idx, pte);
        let entry = pte.addr().as_u64() as *mut PageTable;
        let tablel3: &PageTable = unsafe{&*entry};
        for (idx, pte) in tablel3.iter().enumerate() {
          if pte.flags().contains(PageTableFlags::PRESENT) {
            trace!(" - {}: {:?}", idx, pte);
            let entry = pte.addr().as_u64() as *mut PageTable;
            let tablel2: &PageTable = unsafe{&*entry};
            for (idx, pte) in tablel2.iter().enumerate() {
              if pte.flags().contains(PageTableFlags::PRESENT) {
                trace!(" - - {}: {:?}", idx, pte);
                let entry = pte.addr().as_u64() as *mut PageTable;
                let tablel1: &PageTable = unsafe{&*entry};
                for (idx, pte) in tablel1.iter().enumerate() {
                  if pte.flags().contains(PageTableFlags::PRESENT) {
                    trace!(" - - - {}: {:?}", idx, pte);
                    let entry = pte.addr().as_u64() as *mut PageTable;
                    let tablel1: &PageTable = unsafe{&*entry};
                  }
                }
              }
            }
          }
        }
      }
    }
  })
}

pub fn map_zero(addr: VirtAddr, size: u32) {
  // grab zero_page first, otherwise we get a problem when we grab the lock on the APT
  // below!
  trace!("mapping memory at {:?} ({} pages, {:?})", addr, size, MapType::Zero);
  let zero_page = kinfo().get_zero_page_addr();
  let pm = pager();
  let pagepool = &mut pm.pagepool().clone();
  get_pagemap_mut(|apt| {
    let flags = MapType::Zero.flags();
    let page: Page<Size4KiB> = Page::containing_address(addr);
    for x in 0..size {
      let addr = addr + x as usize * PAGE_SIZE;
      trace!("map zero: {:?}", addr);
      unsafe { apt.map_to(
        page,
        PhysFrame::containing_address(zero_page),
        flags,
        pagepool,
      )} .unwrap().flush()
    }
  })
}

pub fn is_mapped(addr: VirtAddr) -> bool {
  trace!("checking if {:?} is mapped", addr);
  use x86_64::structures::paging::OffsetPageTable;
  get_pagemap(|apt: &OffsetPageTable| {
    apt.translate_addr(addr).is_some()
  })
}

use x86_64::structures::paging::mapper::TranslateResult;

pub fn get_flags(addr: VirtAddr) -> Option<PageTableFlags> {
  trace!("checking if {:?} is mapped", addr);
  use x86_64::structures::paging::OffsetPageTable;
  match get_pagemap(|apt: &OffsetPageTable| {
    apt.translate(addr)
  }) {
    TranslateResult::Frame1GiB{ flags, .. } => Some(flags),
    TranslateResult::Frame2MiB{ flags, .. } => Some(flags),
    TranslateResult::Frame4KiB{ flags, .. } => Some(flags),
    TranslateResult::InvalidFrameAddress(..) => None,
    TranslateResult::PageNotMapped => None,
  }
}

pub fn map(base_addr: VirtAddr, pl: &[PhysAddr], mt: MapType) {
  trace!("mapping memory at {:?} ({} pages, {:?})", base_addr, pl.len(), mt);
  let pm = pager();
  let pagepool = &mut pm.pagepool().clone();
  let flags = mt.flags();
  get_pagemap_mut(|apt| {
    trace!("effective flags: {:?}, {:#064b}", flags, flags.bits());
    for x in 0..pl.len() {
      let addr: VirtAddr = if mt == MapType::Stack {
        base_addr - x * PAGE_SIZE
      } else {
        base_addr + x * PAGE_SIZE
      };
      trace!("map: {:?}", addr);
      let page: Page<Size4KiB> = Page::containing_address(addr);
      unsafe { apt.map_to(page, PhysFrame::containing_address(pl[x]), flags, pagepool) }
        .expect("must not fail map").flush();
    }
  });
  assert_eq!(flags, get_flags(base_addr).expect("must have pagetable flags"));
}

pub fn unmap(base_addr: VirtAddr, pl_size: usize, mt: MapType) {
  trace!("unmapping memory at {:?} ({} pages, {:?})", base_addr, pl_size, mt);
  get_pagemap_mut(|apt| {
    for x in 0..pl_size {
      let addr: VirtAddr = if mt == MapType::Stack {
        base_addr - x * PAGE_SIZE
      } else {
        base_addr + x * PAGE_SIZE
      };
      trace!("unmap: {:?}", addr);
      let page: Page<Size4KiB> = Page::containing_address(addr);
      apt.unmap(page).expect("unmap failed").1.flush();
    }
  })
}

pub fn update_flags(addr: VirtAddr, mt: MapType) {
  trace!("updating flags of memory at {:?} to {:?}", addr, mt);
  assert!(is_mapped(addr), "page must be mapped: {:?}", addr);
  get_pagemap_mut(|apt| {
    let page: Page<Size4KiB> = Page::containing_address(addr);
    unsafe { apt.update_flags(page, mt.flags()).expect("update_flags failed").flush() }
  })
}