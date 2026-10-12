#include "drivers/vga.h"
#include "arch/io.h"

#define VGA_BUFFER_ADDR 0xB8000
#define COM1_PORT       0x3F8

static volatile uint16_t *const vga_buffer = (volatile uint16_t *)VGA_BUFFER_ADDR;
static uint8_t vga_col = 0;
static uint8_t vga_row = 0;
static uint8_t vga_current_color = 0x07; // Light grey on black

// Update hardware text-mode blinking cursor
static void update_hardware_cursor(void) {
    uint16_t position = (uint16_t)(vga_row * VGA_WIDTH + vga_col);

    outb(0x3D4, 0x0F);
    outb(0x3D5, (uint8_t)(position & 0xFF));
    outb(0x3D4, 0x0E);
    outb(0x3D5, (uint8_t)((position >> 8) & 0xFF));
}

// Initialize COM1 serial port for auxiliary console output
static void serial_init(void) {
    outb(COM1_PORT + 1, 0x00); // Disable interrupts
    outb(COM1_PORT + 3, 0x80); // Enable DLAB (set baud rate divisor)
    outb(COM1_PORT + 0, 0x03); // Divisor 3 (lo byte) -> 38400 baud
    outb(COM1_PORT + 1, 0x00); //           (hi byte)
    outb(COM1_PORT + 3, 0x03); // 8 bits, no parity, 1 stop bit
    outb(COM1_PORT + 2, 0xC7); // Enable FIFO, clear, 14-byte threshold
    outb(COM1_PORT + 4, 0x0B); // IRQs enabled, RTS/DSR set
}

// Send single character to COM1 serial port
static void serial_putchar(char c) {
    while ((inb(COM1_PORT + 5) & 0x20) == 0);
    outb(COM1_PORT, (uint8_t)c);
}

// Scroll screen up by 1 line
static void vga_scroll(void) {
    for (size_t y = 0; y < VGA_HEIGHT - 1; y++) {
        for (size_t x = 0; x < VGA_WIDTH; x++) {
            vga_buffer[y * VGA_WIDTH + x] = vga_buffer[(y + 1) * VGA_WIDTH + x];
        }
    }

    // Clear bottom row
    uint16_t blank = vga_entry(' ', vga_current_color);
    for (size_t x = 0; x < VGA_WIDTH; x++) {
        vga_buffer[(VGA_HEIGHT - 1) * VGA_WIDTH + x] = blank;
    }

    vga_row = VGA_HEIGHT - 1;
}

void vga_init(void) {
    serial_init();
    vga_set_color(VGA_COLOR_LIGHT_GREY, VGA_COLOR_BLACK);
    vga_clear();
}

void vga_set_color(enum vga_color fg, enum vga_color bg) {
    vga_current_color = vga_entry_color(fg, bg);
}

void vga_set_raw_color(uint8_t color) {
    vga_current_color = color;
}

uint8_t vga_get_color(void) {
    return vga_current_color;
}

void vga_clear(void) {
    uint16_t blank = vga_entry(' ', vga_current_color);
    for (size_t i = 0; i < VGA_WIDTH * VGA_HEIGHT; i++) {
        vga_buffer[i] = blank;
    }
    vga_col = 0;
    vga_row = 0;
    update_hardware_cursor();
}

void vga_putchar(char c) {
    // Send to serial port
    if (c == '\n') {
        serial_putchar('\r');
    }
    serial_putchar(c);

    // Handle control characters
    if (c == '\n') {
        vga_col = 0;
        vga_row++;
    } else if (c == '\r') {
        vga_col = 0;
    } else if (c == '\t') {
        uint8_t tab_stop = (uint8_t)((vga_col + 4) & ~3);
        if (tab_stop >= VGA_WIDTH) {
            vga_col = 0;
            vga_row++;
        } else {
            vga_col = tab_stop;
        }
    } else if (c == '\b') {
        if (vga_col > 0) {
            vga_col--;
            vga_buffer[vga_row * VGA_WIDTH + vga_col] = vga_entry(' ', vga_current_color);
        }
    } else {
        vga_buffer[vga_row * VGA_WIDTH + vga_col] = vga_entry((unsigned char)c, vga_current_color);
        vga_col++;
        if (vga_col >= VGA_WIDTH) {
            vga_col = 0;
            vga_row++;
        }
    }

    if (vga_row >= VGA_HEIGHT) {
        vga_scroll();
    }

    update_hardware_cursor();
}

void vga_puts(const char *str) {
    if (!str) return;
    while (*str) {
        vga_putchar(*str++);
    }
}

void vga_set_cursor(uint8_t col, uint8_t row) {
    if (col >= VGA_WIDTH) col = VGA_WIDTH - 1;
    if (row >= VGA_HEIGHT) row = VGA_HEIGHT - 1;
    vga_col = col;
    vga_row = row;
    update_hardware_cursor();
}

void vga_get_cursor(uint8_t *col, uint8_t *row) {
    if (col) *col = vga_col;
    if (row) *row = vga_row;
}
