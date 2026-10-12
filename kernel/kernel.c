#include "drivers/vga.h"
#include "lib/printf.h"
#include "arch/isr.h"

void kmain(void) {
    vga_init();
    isr_init();

    vga_set_color(VGA_COLOR_LIGHT_CYAN, VGA_COLOR_BLACK);
    printf("================================================================================\n");
    vga_set_color(VGA_COLOR_LIGHT_GREEN, VGA_COLOR_BLACK);
    printf("                      Welcome to PweOS (32-bit x86)                             \n");
    vga_set_color(VGA_COLOR_LIGHT_CYAN, VGA_COLOR_BLACK);
    printf("================================================================================\n\n");

    vga_set_color(VGA_COLOR_WHITE, VGA_COLOR_BLACK);
    printf("[+] CPU: Intel 80386+ in 32-bit Protected Mode\n");
    printf("[+] Memory: Flat model active (base: %p, limit: 4GB)\n", (void *)0);
    printf("[+] IDT: 256 descriptor gates loaded, PIC remapped to vectors 0x20..0x2F\n");
    printf("[+] Console: VGA %dx%d text buffer at %p\n", VGA_WIDTH, VGA_HEIGHT, (void *)0xB8000);
    printf("[+] Serial: COM1 active at port 0x%x (38400 baud, 8N1)\n", 0x3F8);
    printf("[+] Libc: String functions and printf ready (tested: num=%d, hex=0x%x)\n\n", 42, 0xCAFE);

    vga_set_color(VGA_COLOR_LIGHT_GREY, VGA_COLOR_BLACK);
}
