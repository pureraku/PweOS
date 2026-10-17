#include <limits.h>
#include <stdio.h>
#include <stdlib.h>

static int parse_number(const char *text, int *value) {
    char *end;
    long parsed = strtol(text, &end, 10);
    if (end == text || *end || parsed < INT_MIN || parsed > INT_MAX) return 0;
    *value = (int)parsed;
    return 1;
}

int main(int argc, char **argv) {
    if (argc < 4) {
        puts("Usage: calc <number> <+|-|*|/|%> <number>");
        puts("Example: calc 15 * 7");
        return 1;
    }

    int left, right;
    if (!parse_number(argv[1], &left)) {
        printf("Invalid number '%s'\n", argv[1]);
        return 1;
    }
    if (!parse_number(argv[3], &right)) {
        printf("Invalid number '%s'\n", argv[3]);
        return 1;
    }

    if ((argv[2][0] == '/' || argv[2][0] == '%') && right == 0) {
        puts("Error: Division by zero!");
        return 1;
    }

    if (argv[2][1] != '\0') {
        printf("Unknown operator '%s'. Supported: +, -, *, /, %%\n", argv[2]);
        return 1;
    }

    switch (argv[2][0]) {
        case '+':
            printf("%d + %d = %d\n", left, right,
                   (int)((unsigned int)left + (unsigned int)right));
            break;
        case '-':
            printf("%d - %d = %d\n", left, right,
                   (int)((unsigned int)left - (unsigned int)right));
            break;
        case '*':
            printf("%d * %d = %d\n", left, right,
                   (int)((unsigned int)left * (unsigned int)right));
            break;
        case '/':
            if (left == INT_MIN && right == -1) {
                puts("Error: Result is outside the supported range.");
                return 1;
            }
            printf("%d / %d = %d (remainder %d)\n", left, right,
                   left / right, left % right);
            break;
        case '%':
            if (left == INT_MIN && right == -1) {
                puts("Error: Result is outside the supported range.");
                return 1;
            }
            printf("%d %% %d = %d\n", left, right, left % right);
            break;
        default:
            printf("Unknown operator '%s'. Supported: +, -, *, /, %%\n", argv[2]);
            return 1;
    }
    return 0;
}
