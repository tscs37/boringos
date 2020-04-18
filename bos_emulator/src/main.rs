mod process;
pub use log::{trace,debug,info,warn,error};

extern "C" fn symrfp(sym_type: u16, sym_name: &str) -> *const u8 {
    return 0 as *const u8;
}

fn main() {
    env_logger::Builder::from_default_env()
        //.filter_module("cranelift_codegen", log::LevelFilter::Warn)
        .filter_level(log::LevelFilter::Trace)
        .init();
    info!("BOSEmu v{}.{}.{}", 
        env!("CARGO_PKG_VERSION_MAJOR"), 
        env!("CARGO_PKG_VERSION_MINOR"), 
        env!("CARGO_PKG_VERSION_PATCH"));
    let prc = process::ProcessImage::new_from_file("./res/bosemu_testproc.wasm");
    info!("Emulator returned: {:?}", prc.run(symrfp));
}
