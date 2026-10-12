#include "lib/printf.h"
#include "lib/string.h"
#include "drivers/vga.h"

static void emit_char(char *buf, size_t size, size_t *written, char c) {
    if (buf) {
        if (*written + 1 < size) {
            buf[*written] = c;
        }
    } else {
        vga_putchar(c);
    }
    (*written)++;
}

static void emit_string(char *buf, size_t size, size_t *written, const char *str, int width, bool pad_zero, bool left_align) {
    if (!str) str = "(null)";
    size_t len = strlen(str);
    int pad = (int)width - (int)len;
    char pad_char = pad_zero ? '0' : ' ';

    if (!left_align) {
        for (int i = 0; i < pad; i++) {
            emit_char(buf, size, written, pad_char);
        }
    }

    for (size_t i = 0; i < len; i++) {
        emit_char(buf, size, written, str[i]);
    }

    if (left_align) {
        for (int i = 0; i < pad; i++) {
            emit_char(buf, size, written, ' ');
        }
    }
}

int vsnprintf(char *buffer, size_t size, const char *format, va_list args) {
    size_t written = 0;

    for (size_t i = 0; format[i] != '\0'; i++) {
        if (format[i] != '%') {
            emit_char(buffer, size, &written, format[i]);
            continue;
        }

        i++; // skip '%'
        if (format[i] == '\0') break;

        // Flags
        bool pad_zero = false;
        bool left_align = false;
        if (format[i] == '-') {
            left_align = true;
            i++;
        } else if (format[i] == '0') {
            pad_zero = true;
            i++;
        }

        // Width
        int width = 0;
        while (isdigit(format[i])) {
            width = width * 10 + (format[i] - '0');
            i++;
        }

        // Specifier
        char spec = format[i];
        if (spec == 'c') {
            char c = (char)va_arg(args, int);
            emit_char(buffer, size, &written, c);
        } else if (spec == 's') {
            const char *s = va_arg(args, const char *);
            emit_string(buffer, size, &written, s, width, pad_zero, left_align);
        } else if (spec == 'd' || spec == 'i') {
            int val = va_arg(args, int);
            char num_buf[32];
            itoa(val, num_buf, 10);
            emit_string(buffer, size, &written, num_buf, width, pad_zero, left_align);
        } else if (spec == 'u') {
            unsigned int val = va_arg(args, unsigned int);
            char num_buf[32];
            utoa(val, num_buf, 10);
            emit_string(buffer, size, &written, num_buf, width, pad_zero, left_align);
        } else if (spec == 'x' || spec == 'X') {
            unsigned int val = va_arg(args, unsigned int);
            char num_buf[32];
            utoa(val, num_buf, 16);
            if (spec == 'X') {
                for (size_t j = 0; num_buf[j]; j++) {
                    num_buf[j] = (char)toupper(num_buf[j]);
                }
            }
            emit_string(buffer, size, &written, num_buf, width, pad_zero, left_align);
        } else if (spec == 'p') {
            uintptr_t val = (uintptr_t)va_arg(args, void *);
            char num_buf[32];
            utoa(val, num_buf, 16);
            emit_char(buffer, size, &written, '0');
            emit_char(buffer, size, &written, 'x');
            emit_string(buffer, size, &written, num_buf, width > 2 ? width - 2 : 0, true, left_align);
        } else if (spec == '%') {
            emit_char(buffer, size, &written, '%');
        } else {
            // Unknown specifier
            emit_char(buffer, size, &written, '%');
            emit_char(buffer, size, &written, spec);
        }
    }

    if (buffer && size > 0) {
        if (written < size) {
            buffer[written] = '\0';
        } else {
            buffer[size - 1] = '\0';
        }
    }

    return (int)written;
}

int snprintf(char *buffer, size_t size, const char *format, ...) {
    va_list args;
    va_start(args, format);
    int ret = vsnprintf(buffer, size, format, args);
    va_end(args);
    return ret;
}

int printf(const char *format, ...) {
    va_list args;
    va_start(args, format);
    int ret = vsnprintf(NULL, 0, format, args);
    va_end(args);
    return ret;
}
