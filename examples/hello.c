#include "pweos_libc.h"

int main(int argc, char **argv) {
    static const char message[] = "Hello from C in PweOS user mode!\n";
    pwe_size_t length = strlen(message);
    char *copy = (char *)malloc(length + 1);
    if (!copy) return 1;
    memcpy(copy, message, length + 1);
    pwe_write(1, copy, length);
    free(copy);
    for (int index = 1; index < argc; index++) {
        pwe_write(1, argv[index], strlen(argv[index]));
        pwe_write(1, "\n", 1);
    }
    return 0;
}
