#include "stdlib.h"
#include "ctype.h"
#include "pweos_libc.h"

void exit(int status) {
    pwe_exit(status);
}

void abort(void) {
    pwe_exit(134);
}

static unsigned int digit_value(int c) {
    if (c >= '0' && c <= '9') return (unsigned int)(c - '0');
    if (c >= 'a' && c <= 'z') return (unsigned int)(c - 'a' + 10);
    if (c >= 'A' && c <= 'Z') return (unsigned int)(c - 'A' + 10);
    return (unsigned int)-1;
}

static unsigned long parse_unsigned(const char *text, char **end, int base, int *negative) {
    const char *cursor = text;
    while (isspace((unsigned char)*cursor)) cursor++;
    *negative = 0;
    if (*cursor == '+' || *cursor == '-') {
        *negative = *cursor == '-';
        cursor++;
    }
    if (base == 0) {
        if (cursor[0] == '0' && (cursor[1] == 'x' || cursor[1] == 'X')) {
            base = 16;
            cursor += 2;
        } else if (cursor[0] == '0') {
            base = 8;
        } else {
            base = 10;
        }
    } else if (base == 16 && cursor[0] == '0'
               && (cursor[1] == 'x' || cursor[1] == 'X')) {
        cursor += 2;
    }

    const char *digits = cursor;
    unsigned long value = 0;
    unsigned long limit = (unsigned long)-1;
    int overflow = 0;
    for (;;) {
        unsigned int digit = digit_value((unsigned char)*cursor);
        if (digit >= (unsigned int)base) break;
        if (value > (limit - digit) / (unsigned int)base) overflow = 1;
        else if (!overflow) value = value * (unsigned int)base + digit;
        cursor++;
    }
    if (cursor == digits) cursor = text;
    if (end) *end = (char *)cursor;
    return overflow ? limit : value;
}

unsigned long strtoul(const char *text, char **end, int base) {
    int negative;
    unsigned long value = parse_unsigned(text, end, base, &negative);
    return negative ? 0ul - value : value;
}

long strtol(const char *text, char **end, int base) {
    int negative;
    unsigned long value = parse_unsigned(text, end, base, &negative);
    unsigned long positive_limit = (unsigned long)LONG_MAX;
    unsigned long negative_limit = positive_limit + 1ul;
    if (negative) {
        if (value > negative_limit) return LONG_MIN;
        return value == negative_limit ? LONG_MIN : -(long)value;
    }
    if (value > positive_limit) return LONG_MAX;
    return (long)value;
}

int atoi(const char *text) { return (int)strtol(text, (char **)0, 10); }
long atol(const char *text) { return strtol(text, (char **)0, 10); }
int abs(int value) { return value < 0 ? -value : value; }
long labs(long value) { return value < 0 ? -value : value; }

static void swap_bytes(unsigned char *a, unsigned char *b, size_t size) {
    while (size--) {
        unsigned char byte = *a;
        *a++ = *b;
        *b++ = byte;
    }
}

static void sift_down(unsigned char *base, size_t root, size_t count, size_t size,
                      int (*compare)(const void *, const void *)) {
    if (count < 2) return;
    while (root <= (count - 2) / 2) {
        size_t child = root * 2 + 1;
        if (child + 1 < count
            && compare(base + child * size, base + (child + 1) * size) < 0)
            child++;
        if (compare(base + root * size, base + child * size) >= 0) return;
        swap_bytes(base + root * size, base + child * size, size);
        root = child;
    }
}

void qsort(void *array, size_t count, size_t size,
           int (*compare)(const void *, const void *)) {
    unsigned char *base = (unsigned char *)array;
    if (!base || !compare || size == 0 || count < 2) return;
    for (size_t start = count / 2; start > 0; start--)
        sift_down(base, start - 1, count, size, compare);
    for (size_t end = count - 1; end > 0; end--) {
        swap_bytes(base, base + end * size, size);
        sift_down(base, 0, end, size, compare);
    }
}
