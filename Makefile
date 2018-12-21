.PHONY: all clean kernel release rustup

TARGET = x86_64-boringoscore
CRATE = boringos
QEMU_MEMORY = 512
QEMU_PLATFORM = system-x86_64
BOOTIMG_FILE = target/$(TARGET)/debug/bootimage-$(CRATE).bin
QEMU_OPTIONS = -net none -m $(QEMU_MEMORY) \
	-vga cirrus --enable-kvm --cpu host \
	-drive if=ide,format=raw,file=$(BOOTIMG_FILE) \
	-device isa-debug-exit,iobase=0xf4,iosize=0x04 \
	-serial mon:stdio --no-reboot -d cpu_reset,int

all: kernel bootimage qemu

release: qemu_release

rustup: .rustup
	rustup toolchain add nightly-2018-11-03
	rustup override add nightly-2018-11-03
	rustup component add rust-src
	rustup component add rls-preview rust-analysis
	cargo install cargo-xbuild --force
	cargo install bootimage --version "^0.5.0" --force

clean:
	rm -r target/

bootimage: kernel
	bootimage build --target $(TARGET).json

kernel: 
	#cargo xbuild --target $(TARGET).json

qemu: bootimage
	qemu-$(QEMU_PLATFORM) $(QEMU_OPTIONS) || exit 0

bochs: bootimage
	rm target/$(TARGET)/debug/bootimage-$(CRATE).bin.lock || exit 0
	bochs -f bochs.conf

debug: bootimage
	@qemu-$(QEMU_PLATFORM) $(QEMU_OPTIONS) -S -s || exit 0

gdb: bootimage
	@gdb "$(BOOTIMG_FILE)" -ex "target remote :1234"