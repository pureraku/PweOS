#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <unistd.h>

int main(int argc, char **argv) {
    char text[64];
    char buffer[64];
    FILE *f;
    int i;

    printf("PweOS C Demo\n");
    printf("Arguments: %d\n", argc);

    for (i = 0; i < argc; i++) {
        printf("argv[%d] = %s\n", i, argv[i]);
    }

    /* Basic C operations */
    int a = 10, b = 20;
    printf("Addition: %d\n", a + b);
    printf("Multiplication: %d\n", a * b);

    /* Strings */
    strcpy(text, "Hello from PweOS!");
    printf("String: %s\n", text);
    printf("Length: %u\n", (unsigned)strlen(text));

    /* Heap memory */
    char *memory = malloc(32);
    if (memory != NULL) {
        strcpy(memory, "Dynamic memory works");
        printf("%s\n", memory);
        free(memory);
    }

    /* File I/O */
    f = fopen("/demo.txt", "w");
    if (f != NULL) {
        fprintf(f, "File I/O works!\n");
        fclose(f);

        f = fopen("/demo.txt", "r");
        if (f != NULL) {
            if (fgets(buffer, sizeof(buffer), f)) {
                printf("File says: %s", buffer);
            }
            fclose(f);
        }
        remove("/demo.txt");
    } else {
        printf("File I/O unavailable\n");
    }

    printf("Demo complete!\n");
    return 0;
}
