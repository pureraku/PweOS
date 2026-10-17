#ifndef PWEOS_UNISTD_H
#define PWEOS_UNISTD_H

#include <stddef.h>

typedef int ssize_t;
typedef long off_t;

#define STDIN_FILENO 0
#define STDOUT_FILENO 1
#define STDERR_FILENO 2

ssize_t read(int fd, void *buffer, size_t length);
ssize_t write(int fd, const void *buffer, size_t length);
int close(int fd);
off_t lseek(int fd, off_t offset, int whence);
int unlink(const char *path);
char *getcwd(char *buffer, size_t length);
int isatty(int fd);

#endif
