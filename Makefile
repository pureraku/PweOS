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

ENTRY_OBJ = $(BUILD)/entry.asm.o
ISR_OBJ = $(BUILD)/isr.asm.o
KERNEL_LIB = target/i686-pweos/release/libpweos.a

LDFLAGS = -m elf_i386 --gc-sections --oformat binary -T linker.ld

.PHONY: all run run-fullscreen run-curses run-serial run-headless clean check clippy

all: $(IMAGE)

$(BUILD):
	mkdir -p $(BUILD)

$(BOOT_BIN): boot/boot.asm | $(BUILD)
	$(ASM) $< -f bin -o $@
	test "$$(wc -c < $(BOOT_BIN))" -eq 512

$(ENTRY_OBJ): src/arch/entry.asm | $(BUILD)
	$(ASM) $< -f elf32 -o $@

$(ISR_OBJ): src/arch/isr.asm | $(BUILD)
	$(ASM) $< -f elf32 -o $@

$(KERNEL_LIB): $(wildcard src/*.rs) $(wildcard src/**/*.rs) Cargo.toml targets/i686-pweos.json
	$(CARGO) build --release

$(KERNEL_BIN): $(ENTRY_OBJ) $(ISR_OBJ) $(KERNEL_LIB)
	$(LD) $(LDFLAGS) $^ -o $@

$(IMAGE): $(BOOT_BIN) $(KERNEL_BIN)
	cat $(BOOT_BIN) $(KERNEL_BIN) > $(IMAGE)
	truncate -s 1440K $(IMAGE)

run: $(IMAGE)
	$(QEMU) -drive file=$(IMAGE),format=raw,if=floppy $(QEMU_DISPLAY)

run-fullscreen: $(IMAGE)
	$(QEMU) -drive file=$(IMAGE),format=raw,if=floppy $(QEMU_FULLSCREEN)

run-curses: $(IMAGE)
	$(QEMU) -drive file=$(IMAGE),format=raw,if=floppy -display curses

run-serial: $(IMAGE)
	$(QEMU) -drive file=$(IMAGE),format=raw,if=floppy $(QEMU_DISPLAY) -serial stdio

run-headless: $(IMAGE)
	$(QEMU) -drive file=$(IMAGE),format=raw,if=floppy -display none -serial stdio

check:
	$(CARGO) check

clippy:
	$(CARGO) clippy

clean:
	rm -rf $(BUILD) target
