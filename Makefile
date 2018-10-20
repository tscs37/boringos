.PHONY: all clean kernel release rustup

TARGET = x86_64-boringoscore
CRATE = boringos
QEMU_MEMORY = 512
QEMU_PLATFORM = system-x86_64

all: kernel bootimage qemu

release: qemu_release

rustup: .rustup
	rustup toolchain add nightly-2018-09-22
	rustup override add nightly-2018-09-22
	rustup component add rust-src
	rustup component add rls-preview rust-analysis
	cargo install cargo-xbuild --force
	cargo install bootimage --version "^0.5.0" --force

clean:
	rm -r target/

bootimage: kernel
	bootimage build --target $(TARGET).json

kernel: 
	cargo xbuild --target $(TARGET).json

qemu: bootimage
	qemu-$(QEMU_PLATFORM) -cpu qemu64 \
		-net none -m $(QEMU_MEMORY) \
		-vga cirrus --enable-kvm --cpu host \
		-no-shutdown -no-reboot \
		-drive if=ide,format=raw,file=target/$(TARGET)/debug/bootimage-$(CRATE).bin \
		-device isa-debug-exit,iobase=0xf4,iosize=0x04 \
		-serial mon:stdio -s || exit 0

qemu_no_vga: bootimage
	qemu-$(QEMU_PLATFORM) -cpu qemu64 \
		-net none -m $(QEMU_MEMORY) \
		-display none --enable-kvm --cpu host \
		-no-shutdown -no-reboot \
		-drive if=ide,format=raw,file=target/$(TARGET)/debug/bootimage-$(CRATE).bin \
		-device isa-debug-exit,iobase=0xf4,iosize=0x04 \
		-serial mon:stdio -s || exit 0