
use x86_64::structures::idt::PageFaultErrorCode;
use crate::vmem::{mapper::map_new, mapper::MapType, PAGE_SIZE};
use crate::vmem::pagetable::Page;
use crate::*;

pub type PFHResult = Result<PFHOkResult, PFHErrResult>;

#[derive(Debug)]
pub enum PFHOkResult {
  Mapped,
}

#[derive(Debug)]
pub enum PFHErrResult {
  NoneError,
  InvalidAddress(VirtAddr),
}

impl From<core::option::NoneError> for PFHErrResult {
  fn from(f: core::option::NoneError) -> PFHErrResult { PFHErrResult::NoneError }
}

impl Into<PFHResult> for PFHErrResult {
  fn into(self) -> PFHResult {
    Err(self)
  }
}

impl Into<PFHResult> for PFHOkResult {
  fn into(self) -> PFHResult {
    Ok(self)
  }
}

pub fn handle(vaddr: VirtAddr, error_code: PageFaultErrorCode) -> PFHResult {
  assert!(!error_code.contains(PageFaultErrorCode::MALFORMED_TABLE), "Malformed Page Table");
  let caused_by_prot_violation = error_code.contains(PageFaultErrorCode::PROTECTION_VIOLATION);
  let caused_by_write = error_code.contains(PageFaultErrorCode::CAUSED_BY_WRITE);
  let caused_by_usermode = error_code.contains(PageFaultErrorCode::USER_MODE);
  let caused_by_instr_fetch = error_code.contains(PageFaultErrorCode::INSTRUCTION_FETCH);
  let page = Page::containing_address(vaddr);
  let is_kstack = page.start_address() >= VirtAddr::new(crate::vmem::KSTACK_END.try_into().unwrap())
      && page.start_address() <= VirtAddr::new((crate::vmem::KSTACK_START + PAGE_SIZE).try_into().unwrap());
  let is_ustack = page.start_address() >= VirtAddr::new(crate::vmem::STACK_END.try_into().unwrap())
      && page.start_address() <= VirtAddr::new((crate::vmem::STACK_START + PAGE_SIZE).try_into().unwrap());
  let is_datapage = page.start_address() >= VirtAddr::new(crate::vmem::DATA_START.try_into().unwrap())
      && page.start_address() <= VirtAddr::new(crate::vmem::DATA_END.try_into().unwrap());
  let is_codepage = page.start_address() >= VirtAddr::new(crate::vmem::CODE_START.try_into().unwrap())
      && page.start_address() <= VirtAddr::new(crate::vmem::CODE_END.try_into().unwrap());
  let is_kheap = page.start_address() >= VirtAddr::new(crate::vmem::KHEAP_START.try_into().unwrap())
      && page.start_address() <= VirtAddr::new(crate::vmem::KHEAP_END.try_into().unwrap());

  let paddr = VirtAddr::try_new(page.start_address().as_u64());
  let paddr = match paddr {
    Ok(paddr) => paddr,
    Err(_) => { return PFHErrResult::InvalidAddress(page.start_address()).into() }
  };

  if !caused_by_prot_violation {
      if is_kstack {
          if caused_by_instr_fetch {
              panic!("kernel attempted to run instruction from stack");
          }
          debug!("mapping kstack page to {:?}", page.start_address());
          map_new(paddr, MapType::Stack);
          //TODO: adjust kernel stack size
          debug!("mapped, returning...");
          return PFHOkResult::Mapped.into()
      } else if is_kheap {
        if caused_by_instr_fetch {
          panic!("kernel attempted to run code from heap");
        }
        if vmem::mapper::is_mapped(paddr) {
          panic!("fault on already mapped address");
        }
        debug!("mapping kheap page to {:?}", page.start_address());
        map_new(paddr, MapType::Data);
        return PFHOkResult::Mapped.into()
      } else if is_ustack {
          if caused_by_instr_fetch {
              //TODO: kill task instead
              panic!("task attempted to run instruction from stack")
          }
          trace!("page fault in user stack, mapping new pages");
          handle_ustack(paddr, page)
      } else if page.start_address().as_u64() as usize == crate::vmem::KSTACK_GUARD {
          panic!("stack in kernel stack guard");
      } else if is_codepage || is_datapage {
          if caused_by_instr_fetch {
              // Doesn't work?
              panic!("task could not execute in executable memory");
          }
          if kinfo_mut().mapping_task_image(None) {
            handle_new_umemory(vaddr, page, is_codepage, is_datapage)
          } else if is_datapage {
            handle_running_umemory(vaddr, page)
          } else {
            panic!("tried to access bss or code memory outside mapping zone: {:?}, Data={}, Code={}", paddr, is_datapage, is_codepage);
          }
      } else {
          panic!("cannot map: {:?}", page.start_address());
      }
  } else {
      if is_kstack || is_ustack {
        error!("protection violation in stack area");
      }
      if is_codepage {
        error!("protection violation in code area");
      }
      if is_datapage {
        error!("protection violation in data memory");
      }
      error!(
        "uncovered pagefault occured: {:?} => ({:x}) {:?} \n\n",
        page.start_address(),
        error_code,
        error_code
      );
      panic!("todo: kill task is possible");
  }
}

fn handle_new_umemory(vaddr: VirtAddr, page: Page, is_codepage: bool, is_datapage: bool) -> PFHResult {
  //TODO: check paging mode for paging new task
  //TODO: check if zero page touched
  trace!("checking if the kernel touched memory correctly");
  let expected_vaddr: VirtAddr = {
    let offset: usize = if is_codepage {
      crate::vmem::CODE_START
    } else if is_datapage {
      crate::vmem::DATA_START
    } else {
      panic!("Neither BSS or Code page in BSS or Code only path of page fault");
    };
    let addr: usize = offset + (PAGE_SIZE
                * (kinfo().get_code_memory_ref_size())) as usize;
    VirtAddr::new(addr.try_into().unwrap())
  };
  if expected_vaddr != vaddr {
    error!(
      "wanted kernel to touch {:?} but it touched {:?}",
      expected_vaddr, vaddr
    );
    panic!("kernel touched code memory early, that's nasty");
  }
  let new_page = map_new(vaddr, MapType::Data);
  trace!(
    "mapped new code or data memory, notifying kernel for page {:?}<->{:?}",
    new_page, vaddr
  );
  if is_codepage {
    kinfo_mut().add_code_page(new_page);
  } else if is_datapage {
    kinfo_mut().add_data_page(new_page);
  } else {
    panic!("Neither BSS or Code page in BSS or Code only path of page fault");
  }
  PFHOkResult::Mapped.into()
}

fn handle_running_umemory(vaddr: VirtAddr, page: Page) -> PFHResult {
  //TODO: check paging mode for paging new task
  //TODO: check if zero page touched
  trace!("checking if the kernel touched memory correctly");
  let expected_vaddr = VirtAddr::new((
    crate::vmem::DATA_START
        + (PAGE_SIZE
            * (kinfo().get_data_memory_ref_size()))
    ) as u64);
  if expected_vaddr != vaddr {
    error!(
      "wanted task to touch {:?} but it touched {:?}",
      expected_vaddr, vaddr
    );
    panic!("task touched data memory early, that's nasty");
  }
  let new_page = map_new(vaddr, MapType::Data);
  trace!(
    "mapped new data memory, notifying kernel for page {:?}<->{:?}",
    new_page, vaddr
  );
  kinfo_mut().add_data_page(new_page);
  PFHOkResult::Mapped.into()
}

fn handle_ustack(vaddr: VirtAddr, page: Page) -> PFHResult {
  trace!("checking if the task touched stack correctly");
  let stack_size_org = kinfo().get_stack_memory_ref_size() + 1 - 2;
  let stack_size_org = stack_size_org + 1; // Adjust for 0-base
  let stack_size = stack_size_org - 2; // Allow touching up to 2 pages early
  let expected_vaddr = VirtAddr::new((
    crate::vmem::STACK_START
      - (PAGE_SIZE
        * (stack_size))
  ) as u64);
  let laststack_vaddr = VirtAddr::new((
    crate::vmem::STACK_START
      - (PAGE_SIZE
        * (stack_size_org))
  ) as u64);
  if expected_vaddr <= vaddr {
    error!(
      "wanted task to touch {:?} but it touched {:?}",
      laststack_vaddr, vaddr
    );
    panic!("task touched stack memory early, that's nasty");
  }
  let diff_pages = 
    (laststack_vaddr.as_u64() - vaddr.as_u64()) / PAGE_SIZE as u64 + 1;
  if diff_pages > 1 {
    // sometimes rust jumps a page or two ahead, we yell at it but allow it
    // TODO: include page jumping in task statistics and kill process if it does this too often
    warn!("task jumped {} pages instead of 1, wanted {:?} but got {:?}", 
      diff_pages, laststack_vaddr, vaddr);
    for x in 0..diff_pages {
      let tar_addr: VirtAddr = laststack_vaddr - x as usize * PAGE_SIZE;
      trace!("mapping user stack page to {:#018x}", tar_addr.as_u64());
      let new_page = map_new(tar_addr, MapType::Stack);
      kinfo_mut().add_stack_page(new_page);
    }
  } else if diff_pages == 1 {
    trace!("mapping user stack page to {:?}", page.start_address());
    let new_page = map_new(vaddr, MapType::Stack);
    kinfo_mut().add_stack_page(new_page);
  } else {
    panic!("page fault on supposedly mapped stack");
  }
  PFHOkResult::Mapped.into()
}