#include "math.h"
#include "stdlib.h"
#include "ctype.h"
#include "errno.h"

long double ldexpl(long double value, int exponent) {
    if (value == 0.0L || value != value) return value;
    if (exponent > 20000) return value < 0.0L ? -__builtin_huge_vall() : __builtin_huge_vall();
    if (exponent < -20000) return value < 0.0L ? -0.0L : 0.0L;
    while (exponent > 0) {
        value *= 2.0L;
        exponent--;
    }
    while (exponent < 0) {
        value *= 0.5L;
        exponent++;
    }
    return value;
}

double ldexp(double value, int exponent) {
    return (double)ldexpl((long double)value, exponent);
}

static long double scale_decimal(long double value, int exponent) {
    if (exponent > 6000) return value < 0.0L ? -__builtin_huge_vall() : __builtin_huge_vall();
    if (exponent < -6000) return value < 0.0L ? -0.0L : 0.0L;
    while (exponent > 0) {
        value *= 10.0L;
        exponent--;
    }
    while (exponent < 0) {
        value *= 0.1L;
        exponent++;
    }
    return value;
}

long double strtold(const char *text, char **end) {
    const char *cursor = text;
    while (isspace((unsigned char)*cursor)) cursor++;
    int negative = 0;
    if (*cursor == '+' || *cursor == '-') {
        negative = *cursor == '-';
        cursor++;
    }

    long double value = 0.0L;
    int found_digit = 0;
    int started = 0;
    int stored_digits = 0;
    int input_digits = 0;
    int integer_digits = 0;
    int first_nonzero = -1;
    int after_point = 0;
    for (;;) {
        if (*cursor == '.' && !after_point) {
            after_point = 1;
            cursor++;
            continue;
        }
        if (*cursor < '0' || *cursor > '9') break;
        int digit = *cursor++ - '0';
        found_digit = 1;
        if (!after_point) integer_digits++;
        if (digit != 0 || started) {
            if (!started) {
                started = 1;
                first_nonzero = input_digits;
            }
            if (stored_digits < 19) {
                value = value * 10.0L + digit;
                stored_digits++;
            }
        }
        input_digits++;
    }
    if (!found_digit) {
        if (end) *end = (char *)text;
        return 0.0L;
    }

    int exponent10 = started ? integer_digits - first_nonzero - stored_digits : 0;
    if (*cursor == 'e' || *cursor == 'E') {
        const char *exponent_start = cursor;
        cursor++;
        int exponent_negative = 0;
        if (*cursor == '+' || *cursor == '-') {
            exponent_negative = *cursor == '-';
            cursor++;
        }
        if (*cursor >= '0' && *cursor <= '9') {
            int explicit_exponent = 0;
            while (*cursor >= '0' && *cursor <= '9') {
                if (explicit_exponent < 10000)
                    explicit_exponent = explicit_exponent * 10 + (*cursor - '0');
                cursor++;
            }
            exponent10 += exponent_negative ? -explicit_exponent : explicit_exponent;
        } else {
            cursor = exponent_start;
        }
    }

    if (!started) value = 0.0L;
    else value = scale_decimal(value, exponent10);
    if (negative) value = -value;
    if (started && (value == 0.0L || value == __builtin_huge_vall()
                    || value == -__builtin_huge_vall())) errno = ERANGE;
    if (end) *end = (char *)cursor;
    return value;
}

double strtod(const char *text, char **end) {
    long double parsed = strtold(text, end);
    double result = (double)parsed;
    if (parsed != 0.0L
        && (result == 0.0 || result == __builtin_huge_val()
            || result == -__builtin_huge_val())) errno = ERANGE;
    return result;
}

float strtof(const char *text, char **end) {
    long double parsed = strtold(text, end);
    float result = (float)parsed;
    if (parsed != 0.0L
        && (result == 0.0f || result == __builtin_huge_valf()
            || result == -__builtin_huge_valf())) errno = ERANGE;
    return result;
}
