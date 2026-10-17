#ifndef PWEOS_STRING_H
#define PWEOS_STRING_H

#include <stddef.h>
#include "pweos_libc.h"

void *memchr(const void *memory, int character, size_t length);
size_t strnlen(const char *text, size_t maximum);

#endif
