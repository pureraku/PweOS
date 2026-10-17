#ifndef PWEOS_STDLIB_H
#define PWEOS_STDLIB_H

#include <stddef.h>
#include <limits.h>

#define EXIT_SUCCESS 0
#define EXIT_FAILURE 1
#define RAND_MAX 32767

void *malloc(size_t size);
void *calloc(size_t count, size_t size);
void *realloc(void *pointer, size_t size);
void free(void *pointer);
void exit(int status) __attribute__((noreturn));
void abort(void) __attribute__((noreturn));
char *getenv(const char *name);
char *realpath(const char *path, char *resolved_path);

long strtol(const char *text, char **end, int base);
unsigned long strtoul(const char *text, char **end, int base);
long long strtoll(const char *text, char **end, int base);
unsigned long long strtoull(const char *text, char **end, int base);
double strtod(const char *text, char **end);
float strtof(const char *text, char **end);
long double strtold(const char *text, char **end);
int atoi(const char *text);
long atol(const char *text);
int abs(int value);
long labs(long value);
void qsort(void *base, size_t count, size_t size,
           int (*compare)(const void *, const void *));

#endif
