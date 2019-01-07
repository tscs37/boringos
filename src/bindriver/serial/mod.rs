use log::{Level, LevelFilter, Metadata, Record};
use spin::Mutex;
use uart_16550::SerialPort;

pub type SafeSerialPort = Mutex<SerialPort>;

lazy_static! {
    pub static ref SERIAL1: SafeSerialPort = {
        let mut serial_port = SerialPort::new(0x3F8);
        serial_port.init();
        Mutex::new(serial_port)
    };
}

pub fn init() {
    ::log::set_logger(&SERIAL1).expect("could not setup logging");
    ::log::set_max_level(LevelFilter::Trace);
}

impl ::log::Log for SERIAL1 {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info ||
        (metadata.level() <= Level::Trace && !(false
            // put in blacklisted debug modules here
            // modules listed here are only logged for Info Level or higher
            || metadata.target() == "slabmalloc"
            || metadata.target() == "boringos::vmem::pagelist"
            || metadata.target() == "boringos::vmem::pagetable"
            || metadata.target() == "boringos::bindriver::cpu::idt"
            || metadata.target() == "boringos::vmem::mapper"
            || metadata.target() == "boringos::vmem::pagelist::pagelist_ng"
            || metadata.target() == "boringos::common::kinfo"
            || metadata.target() == "boringos::vmem::faulth"
            || metadata.target() == "boringos::vmem"
            //|| metadata.target() == "boringos::process_manager::state"
            //|| metadata.target() == "boringos::process_manager::memory"
        ))
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            use core::fmt::Write;
            unsafe { self.force_unlock() };
            self.try_lock()
                .and_then(|mut fmt| {
                    fmt.write_fmt(format_args!(
                        "{:6} {:>30}~{:04} - {}\n",
                        record.level(),
                        record
                            .module_path()
                            .expect("need module path to log properly")
                            .trim_start_matches("boringos::"),
                        record.line().expect("need line to log properly"),
                        record.args(),
                    )).ok()
                }).expect("serial did not print");
        }
    }

    fn flush(&self) {}
}
