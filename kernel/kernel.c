#include "drivers/vga.h"
#include "drivers/timer.h"
#include "drivers/keyboard.h"
#include "mm/heap.h"
#include "fs/ramfs.h"
#include "shell/shell.h"
#include "lib/printf.h"
#include "arch/isr.h"

void kmain(void) {
    vga_init();
    isr_init();
    heap_init(HEAP_START_ADDR, HEAP_INITIAL_SIZE);
    ramfs_init();
    timer_init(100);
    keyboard_init();

    // Enable CPU hardware interrupts
    __asm__ volatile ("sti");

    vga_set_color(VGA_COLOR_LIGHT_CYAN, VGA_COLOR_BLACK);
    printf("================================================================================\n");
    vga_set_color(VGA_COLOR_LIGHT_GREEN, VGA_COLOR_BLACK);
    printf("                      Welcome to PweOS (32-bit x86)                             \n");
    vga_set_color(VGA_COLOR_LIGHT_CYAN, VGA_COLOR_BLACK);
    printf("================================================================================\n\n");

    vga_set_color(VGA_COLOR_WHITE, VGA_COLOR_BLACK);
    printf("[+] CPU: Intel 80386+ in 32-bit Protected Mode (IF=1 interrupts active)\n");
    printf("[+] Memory: Flat model active (base: %p, limit: 4GB)\n", (void *)0);
    printf("[+] Heap: Dynamic allocator initialized at %p (size: %u KB)\n", (void *)HEAP_START_ADDR, (unsigned int)(HEAP_INITIAL_SIZE / 1024));
    printf("[+] RAMFS: In-memory filesystem initialized (%u default files mounted)\n", (unsigned int)ramfs_count());
    printf("[+] IDT: 256 descriptor gates loaded, PIC remapped to vectors 0x20..0x2F\n");
    printf("[+] PIT: Timer running at %d Hz (IRQ0 active)\n", 100);
    printf("[+] Keyboard: PS/2 controller driver online (IRQ1 active)\n");
    printf("[+] Console: VGA %dx%d text buffer at %p\n", VGA_WIDTH, VGA_HEIGHT, (void *)0xB8000);
    printf("[+] Serial: COM1 active at port 0x%x (38400 baud, 8N1)\n\n", 0x3F8);

    shell_init();
    shell_run();
}
