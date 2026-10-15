ASM = nasm
LD = ld.lld
CARGO = cargo
QEMU = qemu-system-i386

UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Darwin)
    QEMU_DISPLAY ?= -display cocoa,zoom-to-fit=on
    QEMU_FULLSCREEN ?= -display cocoa,zoom-to-fit=on,full-screen=on
else
    QEMU_DISPLAY ?= -display default
    QEMU_FULLSCREEN ?= -full-screen
endif

BUILD = build
BOOT_BIN = $(BUILD)/boot.bin
KERNEL_BIN = $(BUILD)/kernel.bin
IMAGE = $(BUILD)/pweos.img
VIDEO_IMAGE = $(BUILD)/videos.img
VIDEO_DRIVE = -drive file=$(VIDEO_IMAGE),format=raw,if=ide,index=0

ENTRY_OBJ = $(BUILD)/entry.asm.o
ISR_OBJ = $(BUILD)/isr.asm.o
KERNEL_LIB = target/i686-pweos/release/libpweos.a

LDFLAGS = -m elf_i386 --gc-sections --oformat binary -T linker.ld

.PHONY: all run run-fullscreen run-curses run-serial run-headless clean check clippy

all: $(IMAGE) $(VIDEO_IMAGE)

$(BUILD):
	mkdir -p $(BUILD)

$(BOOT_BIN): boot/boot.asm | $(BUILD)
	$(ASM) $< -f bin -o $@
	test "$$(wc -c < $(BOOT_BIN))" -eq 512

$(ENTRY_OBJ): src/arch/entry.asm | $(BUILD)
	$(ASM) $< -f elf32 -o $@

$(ISR_OBJ): src/arch/isr.asm | $(BUILD)
	$(ASM) $< -f elf32 -o $@

$(KERNEL_LIB): $(wildcard src/*.rs) $(wildcard src/**/*.rs) $(wildcard assets/*.rgb332) $(wildcard assets/*.pwevid) build.rs Cargo.toml targets/i686-pweos.json
	$(CARGO) build --release

$(KERNEL_BIN): $(ENTRY_OBJ) $(ISR_OBJ) $(KERNEL_LIB)
	$(LD) $(LDFLAGS) $^ -o $@
	test "$$(wc -c < $@)" -le 458752

$(IMAGE): $(BOOT_BIN) $(KERNEL_BIN)
	cat $(BOOT_BIN) $(KERNEL_BIN) > $(IMAGE)
	truncate -s 1440K $(IMAGE)

$(VIDEO_IMAGE): $(wildcard assets/*.pwevid) tools/pack_videos.py | $(BUILD)
	python3 tools/pack_videos.py assets $(VIDEO_IMAGE)

run: $(IMAGE) $(VIDEO_IMAGE)
	$(QEMU) -drive file=$(IMAGE),format=raw,if=floppy $(VIDEO_DRIVE) $(QEMU_DISPLAY)

run-fullscreen: $(IMAGE) $(VIDEO_IMAGE)
	$(QEMU) -drive file=$(IMAGE),format=raw,if=floppy $(VIDEO_DRIVE) $(QEMU_FULLSCREEN)

run-curses: $(IMAGE) $(VIDEO_IMAGE)
	$(QEMU) -drive file=$(IMAGE),format=raw,if=floppy $(VIDEO_DRIVE) -display curses

run-serial: $(IMAGE) $(VIDEO_IMAGE)
	$(QEMU) -drive file=$(IMAGE),format=raw,if=floppy $(VIDEO_DRIVE) $(QEMU_DISPLAY) -serial stdio

run-headless: $(IMAGE) $(VIDEO_IMAGE)
	$(QEMU) -drive file=$(IMAGE),format=raw,if=floppy $(VIDEO_DRIVE) -display none -serial stdio

check:
	$(CARGO) check

clippy:
	$(CARGO) clippy

clean:
	rm -rf $(BUILD) target
