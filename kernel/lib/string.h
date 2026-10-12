#ifndef LIB_STRING_H
#define LIB_STRING_H

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

size_t strlen(const char *str);
int strcmp(const char *s1, const char *s2);
int strncmp(const char *s1, const char *s2, size_t n);
char *strcpy(char *dest, const char *src);
char *strncpy(char *dest, const char *src, size_t n);
char *strchr(const char *s, int c);
char *strtok_r(char *str, const char *delim, char **saveptr);

void *memset(void *s, int c, size_t n);
void *memcpy(void *dest, const void *src, size_t n);
void *memmove(void *dest, const void *src, size_t n);
int memcmp(const void *s1, const void *s2, size_t n);

int isspace(int c);
int isdigit(int c);
int isalpha(int c);
int toupper(int c);
int tolower(int c);

int atoi(const char *str);
char *itoa(int value, char *str, int base);
char *utoa(unsigned int value, char *str, int base);

#endif // LIB_STRING_H
