// PweOS Kernel Main
void kmain(void) {
    volatile char *vga = (volatile char *)0xB8000;
    const char *str = "PweOS 32-bit Protected Mode Active!";
    int i = 0;
    while (str[i] != '\0') {
        vga[i * 2] = str[i];
        vga[i * 2 + 1] = 0x0A; // Light green text on black
        i++;
    }
}
