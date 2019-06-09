///
/// This crate provides the basic syscalls that BOS implements for the process environment
/// The crate also provides the symbol resolver function (symrf) that processes use
/// to obtain a pointer to the correct function
/// 


use symrfp::SymbolType;

mod kcalls;

// BOS only provides a base set of symbols, to extend this
// list of syscalls, another process must wrap this syscall
pub extern fn symrf(sym_type: u16, sym_name: &str) -> *mut u8 {
  let st = SymbolType::from(sym_type);
  drop(sym_type);
  trace!("looking up symbol {:?}({})", st, sym_name);
  match st {
    Some(st) => {
      match st {
        SymbolType::TestSymbolResolver => {
          42 as *mut u8
        }
        SymbolType::IPC => {
          match sym_name {
            "bos_set_sig_handler" => kcalls::bos_set_sig_handler as *mut u8,
            "bos_sig_handle" => kcalls::bos_sig_handle as *mut u8,
            "bos_log_trace" => kcalls::bos_log_trace as *mut u8,
            "bos_log_trace_fmt" => kcalls::bos_log_trace_fmt as *mut u8,
            "bos_log_debug" => kcalls::bos_log_debug as *mut u8,
            "bos_log_debug_fmt" => kcalls::bos_log_debug_fmt as *mut u8,
            "bos_log_info" => kcalls::bos_log_info as *mut u8,
            "bos_log_info_fmt" => kcalls::bos_log_info_fmt as *mut u8,
            "bos_log_warn" => kcalls::bos_log_warn as *mut u8,
            "bos_log_warn_fmt" => kcalls::bos_log_warn_fmt as *mut u8,
            "bos_log_error" => kcalls::bos_log_error as *mut u8,
            "bos_log_error_fmt" => kcalls::bos_log_error_fmt as *mut u8,
            "bos_raise_page_limit" => kcalls::bos_raise_page_limit as *mut u8,
            "bos_get_page_limit" => kcalls::bos_get_page_limit as *mut u8,
            "bos_get_page_count_data" => kcalls::bos_get_page_count_data as *mut u8,
            "bos_get_page_count_nondata" => kcalls::bos_get_page_count_nondata as *mut u8,
            "bos_yield" => kcalls::bos_yield as *mut u8,
            "bos_spawn_task" => kcalls::bos_spawn_task as *mut u8,
            _ => 0 as *mut u8,
          }
        },
        _ => { panic!("symbol type not allowed yet") }
        //_ => 0 as *mut u8,
      }
    }
    None => 0 as *mut u8,
  }
}