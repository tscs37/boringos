use uart_16550::SerialPort;
use spin::Mutex;
use ::log::{Record, Level, Metadata, LevelFilter};

pub struct SafeSerialPort(Mutex<SerialPort>);

lazy_static! {
    pub static ref SERIAL1: SafeSerialPort = {
        let mut serial_port = SerialPort::new(0x3F8);
        serial_port.init();
        SafeSerialPort(Mutex::new(serial_port))
    };
}

pub fn init() {
    ::log::set_logger(&SERIAL1).expect("could not setup logging");
    ::log::set_max_level(LevelFilter::Debug);
}

impl ::log::Log for SERIAL1 {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Trace &&
        !(
            // put in blacklisted debug modules here
            metadata.target() == "slabmalloc"
        )
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            use ::core::fmt::Write;
            self.0.lock().write_fmt(format_args!("{} {}~{} - {}\n", 
                record.level(),
                record.module_path().expect("need module path to log properly"),
                record.line().expect("need line to log properly"),
                record.args(),
            )).expect("serial did not print");
        }
    }

    fn flush(&self) {}
}