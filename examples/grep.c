#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

static int line_matches(const char *line, size_t length,
                        const char *needle, size_t needle_length) {
    if (needle_length == 0) return 1;
    if (needle_length > length) return 0;
    for (size_t i = 0; i <= length - needle_length; i++) {
        if (memcmp(line + i, needle, needle_length) == 0) return 1;
    }
    return 0;
}

static int print_matches(FILE *input, const char *needle, const char *label) {
    size_t capacity = 128;
    size_t length = 0;
    size_t needle_length = strlen(needle);
    char *line = malloc(capacity);
    if (!line) return 1;

    int status = 0;
    int character;
    while ((character = fgetc(input)) != EOF) {
        if (length + 1 >= capacity) {
            if (capacity > (size_t)-1 / 2) {
                status = 1;
                break;
            }
            size_t next_capacity = capacity * 2;
            char *next_line = realloc(line, next_capacity);
            if (!next_line) {
                status = 1;
                break;
            }
            line = next_line;
            capacity = next_capacity;
        }
        line[length++] = (char)character;
        if (character == '\n') {
            if (line_matches(line, length, needle, needle_length)) {
                if (label) printf("%s:", label);
                if (fwrite(line, 1, length, stdout) != length) status = 1;
            }
            length = 0;
        }
    }

    if (length && line_matches(line, length, needle, needle_length)) {
        if (label) printf("%s:", label);
        if (fwrite(line, 1, length, stdout) != length) status = 1;
    }
    if (ferror(input)) status = 1;
    free(line);
    return status;
}

int main(int argc, char **argv) {
    if (argc < 2) {
        puts("Usage: grep <text> [file...]");
        return 1;
    }

    if (argc == 2) {
        if (isatty(STDIN_FILENO)) {
            puts("Usage: grep <text> [file...] (or provide piped input)");
            return 1;
        }
        return print_matches(stdin, argv[1], NULL);
    }

    int status = 0;
    const char *label = argc > 3 ? "" : NULL;
    for (int index = 2; index < argc; index++) {
        FILE *input = fopen(argv[index], "rb");
        if (!input) {
            printf("grep: '%s' not found\n", argv[index]);
            status = 1;
            continue;
        }
        if (print_matches(input, argv[1], label ? argv[index] : NULL) != 0) status = 1;
        if (fclose(input) != 0) status = 1;
    }
    return status;
}
