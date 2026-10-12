#include "drivers/keyboard.h"
#include "drivers/vga.h"
#include "arch/isr.h"
#include "arch/io.h"
#include "arch/pic.h"

#define KEYBOARD_DATA_PORT    0x60
#define KEYBOARD_STATUS_PORT  0x64
#define COM1_LSR_PORT         0x3FD
#define COM1_DATA_PORT        0x3F8

#define KB_BUFFER_SIZE        128

static volatile char kb_buffer[KB_BUFFER_SIZE];
static volatile size_t kb_head = 0;
static volatile size_t kb_tail = 0;

static bool shift_pressed = false;
static bool caps_lock = false;
static bool extended_prefix = false;

// Scancode Set 1 standard US QWERTY mapping
static const char scancode_ascii_nomod[128] = {
    0,   27,  '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', '-', '=', '\b',
    '\t', 'q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p', '[', ']', '\n',
    0,   'a', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l', ';', '\'', '`',
    0,   '\\', 'z', 'x', 'c', 'v', 'b', 'n', 'm', ',', '.', '/', 0,
    '*', 0,   ' ', 0,   0,   0,   0,   0,   0,   0,   0,   0,   0,
    0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,
    0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0
};

static const char scancode_ascii_shift[128] = {
    0,   27,  '!', '@', '#', '$', '%', '^', '&', '*', '(', ')', '_', '+', '\b',
    '\t', 'Q', 'W', 'E', 'R', 'T', 'Y', 'U', 'I', 'O', 'P', '{', '}', '\n',
    0,   'A', 'S', 'D', 'F', 'G', 'H', 'J', 'K', 'L', ':', '"', '~',
    0,   '|', 'Z', 'X', 'C', 'V', 'B', 'N', 'M', '<', '>', '?', 0,
    '*', 0,   ' ', 0,   0,   0,   0,   0,   0,   0,   0,   0,   0,
    0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,
    0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0
};

static void buffer_push(char c) {
    size_t next_head = (kb_head + 1) % KB_BUFFER_SIZE;
    if (next_head != kb_tail) {
        kb_buffer[kb_head] = c;
        kb_head = next_head;
    }
}

static void keyboard_callback(registers_t *regs) {
    (void)regs;
    uint8_t scancode = inb(KEYBOARD_DATA_PORT);

    if (scancode == 0xE0) {
        extended_prefix = true;
        return;
    }

    if (extended_prefix) {
        extended_prefix = false;
        if (scancode & 0x80) return; // Release

        if (scancode == 0x48)      buffer_push(KEY_UP);
        else if (scancode == 0x50) buffer_push(KEY_DOWN);
        else if (scancode == 0x4B) buffer_push(KEY_LEFT);
        else if (scancode == 0x4D) buffer_push(KEY_RIGHT);
        return;
    }

    // Key release (break code)
    if (scancode & 0x80) {
        uint8_t released = scancode & 0x7F;
        if (released == 0x2A || released == 0x36) { // Left or Right Shift
            shift_pressed = false;
        }
        return;
    }

    // Key press (make code)
    if (scancode == 0x2A || scancode == 0x36) { // Left or Right Shift
        shift_pressed = true;
        return;
    }

    if (scancode == 0x3A) { // Caps Lock
        caps_lock = !caps_lock;
        return;
    }

    if (scancode < 128) {
        char c = 0;
        char c_normal = scancode_ascii_nomod[scancode];
        char c_shift  = scancode_ascii_shift[scancode];

        if (c_normal >= 'a' && c_normal <= 'z') {
            bool uppercase = shift_pressed ^ caps_lock;
            c = uppercase ? c_shift : c_normal;
        } else {
            c = shift_pressed ? c_shift : c_normal;
        }

        if (c != 0) {
            buffer_push(c);
        }
    }
}

void keyboard_init(void) {
    kb_head = 0;
    kb_tail = 0;
    shift_pressed = false;
    caps_lock = false;
    extended_prefix = false;

    // Register IRQ1 handler (vector 33)
    isr_install_handler(33, keyboard_callback);

    // Unmask IRQ1 on Master PIC
    pic_clear_mask(1);
}

bool keyboard_has_char(void) {
    if (kb_head != kb_tail) {
        return true;
    }
    // Check if COM1 serial port has received byte
    if (inb(COM1_LSR_PORT) & 0x01) {
        return true;
    }
    return false;
}

char keyboard_getchar(void) {
    for (;;) {
        // First check serial port
        if (inb(COM1_LSR_PORT) & 0x01) {
            char c = (char)inb(COM1_DATA_PORT);
            if (c == 0x1B) { // ANSI escape sequence
                if (inb(COM1_LSR_PORT) & 0x01) {
                    char next = (char)inb(COM1_DATA_PORT);
                    if (next == '[') {
                        if (inb(COM1_LSR_PORT) & 0x01) {
                            char code = (char)inb(COM1_DATA_PORT);
                            if (code == 'A') return KEY_UP;
                            if (code == 'B') return KEY_DOWN;
                            if (code == 'C') return KEY_RIGHT;
                            if (code == 'D') return KEY_LEFT;
                        }
                    }
                }
            }
            if (c == '\r') c = '\n';
            if (c == 127)  c = '\b'; // Map DEL to backspace
            return c;
        }

        // Next check keyboard buffer
        if (kb_head != kb_tail) {
            char c = kb_buffer[kb_tail];
            kb_tail = (kb_tail + 1) % KB_BUFFER_SIZE;
            return c;
        }

        // Wait for interrupt
        __asm__ volatile ("hlt");
    }
}

size_t keyboard_readline(char *buffer, size_t max_len) {
    if (!buffer || max_len == 0) return 0;
    size_t index = 0;

    while (index < max_len - 1) {
        char c = keyboard_getchar();

        if (c == '\n' || c == '\r') {
            vga_putchar('\n');
            buffer[index] = '\0';
            return index;
        } else if (c == '\b') {
            if (index > 0) {
                index--;
                vga_putchar('\b');
            }
        } else if ((unsigned char)c >= 32 && (unsigned char)c <= 126) {
            buffer[index++] = c;
            vga_putchar(c);
        }
    }

    buffer[index] = '\0';
    vga_putchar('\n');
    return index;
}
