# BOSEmu

A high-level BoringOS Emulator. Unlike the actual kernel, this is a normal userspace program that emulates some
of the interfaces of the BOS kernel. It requires a special driver subsystem that is not compatible with
baremetal to get access to devices and filesystems. Final userspace is compatible to BOS Kernel. Additionally
BOSEmu offers no scheduler, instead, it will use the standard linux thread primitive for internal processes.