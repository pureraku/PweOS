#include "lib/string.h"

size_t strlen(const char *str) {
    if (!str) return 0;
    size_t len = 0;
    while (str[len]) {
        len++;
    }
    return len;
}

int strcmp(const char *s1, const char *s2) {
    while (*s1 && (*s1 == *s2)) {
        s1++;
        s2++;
    }
    return (int)((unsigned char)*s1 - (unsigned char)*s2);
}

int strncmp(const char *s1, const char *s2, size_t n) {
    for (size_t i = 0; i < n; i++) {
        if (s1[i] != s2[i]) {
            return (int)((unsigned char)s1[i] - (unsigned char)s2[i]);
        }
        if (s1[i] == '\0') {
            break;
        }
    }
    return 0;
}

char *strcpy(char *dest, const char *src) {
    char *ret = dest;
    while ((*dest++ = *src++));
    return ret;
}

char *strncpy(char *dest, const char *src, size_t n) {
    size_t i;
    for (i = 0; i < n && src[i] != '\0'; i++) {
        dest[i] = src[i];
    }
    for (; i < n; i++) {
        dest[i] = '\0';
    }
    return dest;
}

char *strchr(const char *s, int c) {
    char ch = (char)c;
    while (*s != ch) {
        if (!*s++) return NULL;
    }
    return (char *)s;
}

char *strtok_r(char *str, const char *delim, char **saveptr) {
    if (!saveptr) return NULL;
    char *s = str ? str : *saveptr;
    if (!s) return NULL;

    // Skip leading delimiters
    while (*s) {
        bool is_delim = false;
        for (const char *d = delim; *d; d++) {
            if (*s == *d) {
                is_delim = true;
                break;
            }
        }
        if (!is_delim) break;
        s++;
    }

    if (!*s) {
        *saveptr = NULL;
        return NULL;
    }

    char *token_start = s;

    // Find end of token
    while (*s) {
        bool is_delim = false;
        for (const char *d = delim; *d; d++) {
            if (*s == *d) {
                is_delim = true;
                break;
            }
        }
        if (is_delim) {
            *s = '\0';
            *saveptr = s + 1;
            return token_start;
        }
        s++;
    }

    *saveptr = NULL;
    return token_start;
}

void *memset(void *s, int c, size_t n) {
    unsigned char *p = (unsigned char *)s;
    unsigned char val = (unsigned char)c;
    for (size_t i = 0; i < n; i++) {
        p[i] = val;
    }
    return s;
}

void *memcpy(void *dest, const void *src, size_t n) {
    unsigned char *d = (unsigned char *)dest;
    const unsigned char *s = (const unsigned char *)src;
    for (size_t i = 0; i < n; i++) {
        d[i] = s[i];
    }
    return dest;
}

void *memmove(void *dest, const void *src, size_t n) {
    unsigned char *d = (unsigned char *)dest;
    const unsigned char *s = (const unsigned char *)src;
    if (d < s) {
        for (size_t i = 0; i < n; i++) {
            d[i] = s[i];
        }
    } else if (d > s) {
        for (size_t i = n; i > 0; i--) {
            d[i - 1] = s[i - 1];
        }
    }
    return dest;
}

int memcmp(const void *s1, const void *s2, size_t n) {
    const unsigned char *p1 = (const unsigned char *)s1;
    const unsigned char *p2 = (const unsigned char *)s2;
    for (size_t i = 0; i < n; i++) {
        if (p1[i] != p2[i]) {
            return (int)p1[i] - (int)p2[i];
        }
    }
    return 0;
}

int isspace(int c) {
    return c == ' ' || c == '\t' || c == '\n' || c == '\r' || c == '\f' || c == '\v';
}

int isdigit(int c) {
    return c >= '0' && c <= '9';
}

int isalpha(int c) {
    return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z');
}

int toupper(int c) {
    if (c >= 'a' && c <= 'z') return c - ('a' - 'A');
    return c;
}

int tolower(int c) {
    if (c >= 'A' && c <= 'Z') return c + ('a' - 'A');
    return c;
}

int atoi(const char *str) {
    if (!str) return 0;
    while (isspace(*str)) str++;
    int sign = 1;
    if (*str == '-') {
        sign = -1;
        str++;
    } else if (*str == '+') {
        str++;
    }
    int res = 0;
    while (isdigit(*str)) {
        res = res * 10 + (*str - '0');
        str++;
    }
    return res * sign;
}

char *utoa(unsigned int value, char *str, int base) {
    if (!str || base < 2 || base > 16) return NULL;
    char buffer[36];
    int pos = 0;

    if (value == 0) {
        str[0] = '0';
        str[1] = '\0';
        return str;
    }

    const char digits[] = "0123456789abcdef";
    while (value > 0) {
        buffer[pos++] = digits[value % (unsigned int)base];
        value /= (unsigned int)base;
    }

    int j = 0;
    for (int i = pos - 1; i >= 0; i--) {
        str[j++] = buffer[i];
    }
    str[j] = '\0';
    return str;
}

char *itoa(int value, char *str, int base) {
    if (!str || base < 2 || base > 16) return NULL;

    if (value < 0 && base == 10) {
        str[0] = '-';
        utoa((unsigned int)(-value), str + 1, base);
        return str;
    }
    return utoa((unsigned int)value, str, base);
}
