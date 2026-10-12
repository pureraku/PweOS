#include "drivers/vga.h"

void kmain(void) {
    vga_init();

    vga_set_color(VGA_COLOR_LIGHT_CYAN, VGA_COLOR_BLACK);
    vga_puts("================================================================================\n");
    vga_set_color(VGA_COLOR_LIGHT_GREEN, VGA_COLOR_BLACK);
    vga_puts("                      Welcome to PweOS (32-bit x86)                             \n");
    vga_set_color(VGA_COLOR_LIGHT_CYAN, VGA_COLOR_BLACK);
    vga_puts("================================================================================\n\n");

    vga_set_color(VGA_COLOR_WHITE, VGA_COLOR_BLACK);
    vga_puts("[+] CPU switched to 32-bit Protected Mode successfully.\n");
    vga_puts("[+] Flat memory segments active (Base=0, Limit=4GB).\n");
    vga_puts("[+] VGA 80x25 text mode driver initialized.\n");
    vga_puts("[+] Serial console COM1 initialized (38400 baud).\n\n");

    vga_set_color(VGA_COLOR_LIGHT_GREY, VGA_COLOR_BLACK);
}
