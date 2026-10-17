#ifndef PWEOS_LIBC_H
#define PWEOS_LIBC_H

#include "pweos.h"

void *memcpy(void *destination, const void *source, pwe_size_t length);
void *memmove(void *destination, const void *source, pwe_size_t length);
void *memset(void *destination, int value, pwe_size_t length);
int memcmp(const void *left, const void *right, pwe_size_t length);
void *memchr(const void *memory, int character, pwe_size_t length);

pwe_size_t strlen(const char *text);
pwe_size_t strnlen(const char *text, pwe_size_t maximum);
int strcmp(const char *left, const char *right);
int strncmp(const char *left, const char *right, pwe_size_t length);
char *strcpy(char *destination, const char *source);
char *strncpy(char *destination, const char *source, pwe_size_t length);
char *strcat(char *destination, const char *source);
char *strchr(const char *text, int character);
char *strrchr(const char *text, int character);
char *strstr(const char *text, const char *needle);
char *strdup(const char *text);

void *malloc(pwe_size_t size);
void *calloc(pwe_size_t count, pwe_size_t size);
void *realloc(void *pointer, pwe_size_t size);
void free(void *pointer);

#endif
