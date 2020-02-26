
use crate::vmem::{mapper::map_new, mapper::MapType, PAGE_SIZE};
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

use crate::bindriver::cpu::PageFaultContext;

pub fn handle(pfc: PageFaultContext) -> PFHResult {
  pfc.assert_valid();

  let vaddr = VirtAddr::try_new(pfc.page().start_address().as_u64());
  let vaddr = match vaddr {
    Ok(vaddr) => vaddr,
    Err(_) => { return PFHErrResult::InvalidAddress(pfc.page().start_address()).into() }
  };

  if !pfc.caused_by_protection_violation() {
      if pfc.is_kstack() {
          if pfc.caused_by_instruction_fetch() {
              panic!("kernel attempted to run instruction from stack: {:?}", pfc);
          }
          debug!("mapping kstack page to {:?}", pfc.page().start_address());
          map_new(vaddr, MapType::Stack);
          //TODO: adjust kernel stack size
          debug!("mapped, returning...");
          return PFHOkResult::Mapped.into()
      } else if pfc.is_kheap() {
        if pfc.caused_by_instruction_fetch() {
          panic!("kernel attempted to run code from heap: {:?}", pfc);
        }
        if vmem::mapper::is_mapped(vaddr) {
          panic!("fault on already mapped address: {:?}", pfc);
        }
        debug!("mapping kheap page to {:?}", pfc.page().start_address());
        map_new(vaddr, MapType::Data);
        return PFHOkResult::Mapped.into()
      } else if pfc.is_ustack() {
          if pfc.caused_by_instruction_fetch() {
              //TODO: kill task instead
              panic!("task attempted to run instruction from stack: {:?}", pfc);
          }
          trace!("page fault in user stack, mapping new pages");
          handle_ustack(pfc)
      } else if pfc.page().start_address().as_u64() as usize == crate::vmem::KSTACK_GUARD {
          panic!("stack in kernel stack guard");
      } else if pfc.is_ucode() || pfc.is_udata() {
          if pfc.caused_by_instruction_fetch() {
              // Doesn't work?
              panic!("task could not execute in executable memory: {:?}", pfc);
          }
          if kinfo_mut().mapping_task_image(None) {
            handle_new_umemory(pfc)
          } else if pfc.is_udata() {
            handle_running_umemory(pfc)
          } else {
            panic!("tried to access bss or code memory outside mapping zone: {:?}, Data={}, Code={}", 
              vaddr, pfc.is_udata(), pfc.is_ucode());
          }
      } else {
          panic!("cannot map: {:?}", pfc);
      }
  } else {
      if pfc.is_kstack() || pfc.is_ustack() {
        error!("protection violation in stack area");
      }
      if pfc.is_ucode() {
        error!("protection violation in code area");
      }
      if pfc.is_udata() {
        error!("protection violation in data memory");
      }
      error!(
        "uncovered pagefault occured: {:?} => ({:x}) {:?} \n\n",
        pfc.page().start_address(),
        pfc.error_code(),
        pfc.error_code()
      );
      panic!("todo: kill task is possible: {:?}", pfc);
  }
}

fn handle_new_umemory(pfc: PageFaultContext) -> PFHResult {
  //TODO: check paging mode for paging new task
  //TODO: check if zero page touched
  trace!("checking if the kernel touched memory correctly");
  let expected_vaddr: VirtAddr = {
    let offset: usize = if pfc.is_ucode() {
      trace!("page fault in code memory");
      crate::vmem::CODE_START
    } else if pfc.is_udata() {
      trace!("page fault in data memory");
      crate::vmem::DATA_START
    } else {
      panic!("Neither Data or Code page on page fault");
    };
    let ref_size = {
      if pfc.is_ucode() {
        kinfo().get_code_memory_ref_size()
      } else if pfc.is_udata() {
        kinfo().get_data_memory_ref_size()
      } else {
        panic!("Neither Data or Code page on page fault");
      }
    };
    let addr: usize = offset + (PAGE_SIZE
                * (ref_size + 1)) as usize;
    VirtAddr::new(addr.try_into().unwrap())
  };
  trace!("expected touch on {:?}, checking", expected_vaddr);
  if expected_vaddr < pfc.fault_address() {
    error!(
      "wanted kernel to touch {:?} but it touched {:?}",
      expected_vaddr, pfc.fault_address()
    );
    panic!("kernel touched code memory early, that's nasty");
  }
  let new_page = map_new(pfc.fault_address(), MapType::Data);
  trace!(
    "mapped new code or data memory, notifying kernel for page {:?}<->{:?}",
    new_page, pfc.fault_address()
  );
  if pfc.is_ucode() {
    kinfo_mut().add_code_page(new_page);
  } else if pfc.is_udata() {
    kinfo_mut().add_data_page(new_page);
  } else {
    panic!("Neither BSS or Code page in BSS or Code only path of page fault");
  }
  PFHOkResult::Mapped.into()
}

fn handle_running_umemory(pfc: PageFaultContext) -> PFHResult {
  //TODO: check paging mode for paging new task
  //TODO: check if zero page touched
  trace!("checking if the kernel touched memory correctly");
  let expected_vaddr = VirtAddr::new((
    crate::vmem::DATA_START
        + (PAGE_SIZE
            * (kinfo().get_data_memory_ref_size()))
    ) as u64);
  if expected_vaddr != pfc.fault_address() {
    error!(
      "wanted task to touch {:?} but it touched {:?}",
      expected_vaddr, pfc.fault_address()
    );
    panic!("task touched data memory early, that's nasty");
  }
  let new_page = map_new(pfc.fault_address(), MapType::Data);
  trace!(
    "mapped new data memory, notifying kernel for page {:?}<->{:?}",
    new_page, pfc.fault_address()
  );
  kinfo_mut().add_data_page(new_page);
  PFHOkResult::Mapped.into()
}

fn handle_ustack(pfc: PageFaultContext) -> PFHResult {
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
  if expected_vaddr <= pfc.fault_address() {
    error!(
      "wanted task to touch {:?} but it touched {:?}",
      laststack_vaddr, pfc.fault_address()
    );
    panic!("task touched stack memory early, that's nasty");
  }
  let diff_pages = 
    (laststack_vaddr.as_u64() - pfc.fault_address().as_u64()) / PAGE_SIZE as u64 + 1;
  if diff_pages > 1 {
    // sometimes rust jumps a page or two ahead, we yell at it but allow it
    // TODO: include page jumping in task statistics and kill process if it does this too often
    warn!("task jumped {} pages instead of 1, wanted {:?} but got {:?}", 
      diff_pages, laststack_vaddr, pfc.fault_address());
    for x in 0..diff_pages {
      let tar_addr: VirtAddr = laststack_vaddr - x as usize * PAGE_SIZE;
      trace!("mapping user stack page to {:#018x}", tar_addr.as_u64());
      let new_page = map_new(tar_addr, MapType::Stack);
      kinfo_mut().add_stack_page(new_page);
    }
  } else if diff_pages == 1 {
    trace!("mapping user stack page to {:?}", pfc.page().start_address());
    let new_page = map_new(pfc.fault_address(), MapType::Stack);
    kinfo_mut().add_stack_page(new_page);
  } else {
    panic!("page fault on supposedly mapped stack");
  }
  PFHOkResult::Mapped.into()
}