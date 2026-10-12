ASM = nasm
CC = clang --target=i686-unknown-none-elf
LD = ld.lld
QEMU = qemu-system-i386

CFLAGS = -ffreestanding -m32 -mgeneral-regs-only -mno-sse -mno-sse2 -mno-mmx -fno-pie -fno-pic -fno-stack-protector -fno-builtin -Wall -Wextra -O2 -Ikernel
LDFLAGS = -m elf_i386 --oformat binary -T kernel/linker.ld

BUILD = build
BOOT_BIN = $(BUILD)/boot.bin
KERNEL_BIN = $(BUILD)/kernel.bin
IMAGE = $(BUILD)/pweos.img

KERNEL_ASM_SRCS = $(wildcard kernel/arch/*.asm)
KERNEL_C_SRCS = $(wildcard kernel/*.c) $(wildcard kernel/**/*.c)
KERNEL_ASM_OBJS = $(patsubst kernel/%.asm, $(BUILD)/%.asm.o, $(KERNEL_ASM_SRCS))
KERNEL_C_OBJS = $(patsubst kernel/%.c, $(BUILD)/%.c.o, $(KERNEL_C_SRCS))

.PHONY: all run run-serial run-headless clean

all: $(IMAGE)

$(BUILD):
	mkdir -p $(BUILD)/arch $(BUILD)/drivers $(BUILD)/lib $(BUILD)/shell $(BUILD)/mm $(BUILD)/fs

$(BOOT_BIN): boot/boot.asm | $(BUILD)
	$(ASM) $< -f bin -o $@
	test "$$(wc -c < $(BOOT_BIN))" -eq 512

$(BUILD)/%.asm.o: kernel/%.asm | $(BUILD)
	mkdir -p $(dir $@)
	$(ASM) $< -f elf32 -o $@

$(BUILD)/%.c.o: kernel/%.c | $(BUILD)
	mkdir -p $(dir $@)
	$(CC) $(CFLAGS) -c $< -o $@

$(KERNEL_BIN): $(KERNEL_ASM_OBJS) $(KERNEL_C_OBJS)
	$(LD) $(LDFLAGS) $^ -o $@

$(IMAGE): $(BOOT_BIN) $(KERNEL_BIN)
	cat $(BOOT_BIN) $(KERNEL_BIN) > $(IMAGE)
	truncate -s 1440K $(IMAGE)

run: $(IMAGE)
	$(QEMU) -drive file=$(IMAGE),format=raw,if=floppy

run-serial: $(IMAGE)
	$(QEMU) -drive file=$(IMAGE),format=raw,if=floppy -serial stdio

run-headless: $(IMAGE)
	$(QEMU) -drive file=$(IMAGE),format=raw,if=floppy -display none -serial stdio

clean:
	rm -rf $(BUILD)
