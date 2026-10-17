#include "pweos_libc.h"

void _start(void) __attribute__((noreturn));
void _start(void) {
    static const char message[] = "Hello from C in PweOS user mode!\n";
    pwe_size_t length = strlen(message);
    char *copy = (char *)malloc(length + 1);
    if (!copy) pwe_exit(1);
    memcpy(copy, message, length + 1);
    pwe_write(1, copy, length);
    free(copy);
    pwe_exit(0);
}
