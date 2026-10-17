#include <stdio.h>
#include <unistd.h>

static int copy_stream(FILE *input) {
    unsigned char buffer[512];
    size_t count;
    while ((count = fread(buffer, 1, sizeof(buffer), input)) != 0) {
        if (fwrite(buffer, 1, count, stdout) != count) return 1;
    }
    return ferror(input) ? 1 : 0;
}

int main(int argc, char **argv) {
    if (argc == 1) {
        if (isatty(STDIN_FILENO)) {
            puts("Usage: cat <file...>");
            return 1;
        }
        return copy_stream(stdin);
    }

    int status = 0;
    for (int index = 1; index < argc; index++) {
        FILE *input = fopen(argv[index], "rb");
        if (!input) {
            printf("cat: '%s' not found\n", argv[index]);
            status = 1;
            continue;
        }
        if (copy_stream(input) != 0) status = 1;
        if (fclose(input) != 0) status = 1;
    }
    return status;
}
