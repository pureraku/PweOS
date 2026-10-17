#ifndef PWEOS_STDIO_H
#define PWEOS_STDIO_H

#include <stdarg.h>
#include <stddef.h>
#include "pweos.h"

typedef struct pwe_file {
    int fd;
    int readable;
    int writable;
    int owns_fd;
    int eof;
    int error;
    int pushed_char;
} FILE;

#define EOF (-1)
#define SEEK_SET PWE_SEEK_SET
#define SEEK_CUR PWE_SEEK_CUR
#define SEEK_END PWE_SEEK_END
#define BUFSIZ 4096

extern FILE *stdin;
extern FILE *stdout;
extern FILE *stderr;

FILE *fopen(const char *path, const char *mode);
FILE *fdopen(int fd, const char *mode);
FILE *freopen(const char *path, const char *mode, FILE *stream);
int fclose(FILE *stream);
int fflush(FILE *stream);
size_t fread(void *buffer, size_t size, size_t count, FILE *stream);
size_t fwrite(const void *buffer, size_t size, size_t count, FILE *stream);
int fseek(FILE *stream, long offset, int origin);
long ftell(FILE *stream);
void rewind(FILE *stream);
int fgetc(FILE *stream);
int getc(FILE *stream);
int getchar(void);
int ungetc(int character, FILE *stream);
char *fgets(char *buffer, int length, FILE *stream);
int fputc(int character, FILE *stream);
int putchar(int character);
int fputs(const char *text, FILE *stream);
int puts(const char *text);
int feof(FILE *stream);
int ferror(FILE *stream);
void clearerr(FILE *stream);
int remove(const char *path);
void perror(const char *prefix);
int vfprintf(FILE *stream, const char *format, va_list args);
int fprintf(FILE *stream, const char *format, ...);
int printf(const char *format, ...);
int vsnprintf(char *buffer, size_t length, const char *format, va_list args);
int snprintf(char *buffer, size_t length, const char *format, ...);
int vsprintf(char *buffer, const char *format, va_list args);
int sprintf(char *buffer, const char *format, ...);

#endif
