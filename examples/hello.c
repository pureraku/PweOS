#include "pweos_libc.h"
#include "stdio.h"

int main(int argc, char **argv) {
    static const char message[] = "Hello from C in PweOS user mode!\n";
    pwe_size_t length = strlen(message);
    char *copy = (char *)malloc(length + 1);
    if (!copy) return 1;
    memcpy(copy, message, length + 1);
    printf("%s", copy);
    free(copy);
    for (int index = 1; index < argc; index++) {
        printf("%s\n", argv[index]);
    }
    return 0;
}
