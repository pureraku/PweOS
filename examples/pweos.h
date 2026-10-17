#ifndef PWEOS_H
#define PWEOS_H

#include "errno.h"

typedef unsigned int pwe_size_t;
typedef int pwe_ssize_t;

enum {
    PWE_SYS_EXIT = 0,
    PWE_SYS_WRITE = 1,
    PWE_SYS_OPEN = 2,
    PWE_SYS_READ = 3,
    PWE_SYS_CLOSE = 4,
    PWE_SYS_BRK = 5,
    PWE_SYS_FWRITE = 6,
    PWE_SYS_LSEEK = 7,
    PWE_SYS_UNLINK = 8,
    PWE_SYS_GETCWD = 9,
    PWE_SYS_GETTIMEOFDAY = 10,
    PWE_SYS_CONSOLE = 11,
};

enum {
    PWE_CONSOLE_CLEAR = 0,
    PWE_CONSOLE_SET_CURSOR = 1,
};

enum {
    PWE_O_RDONLY = 0,
    PWE_O_WRONLY = 1,
    PWE_O_RDWR = 2,
    PWE_O_CREAT = 0x0040,
    PWE_O_TRUNC = 0x0200,
    PWE_O_APPEND = 0x0400,
};

enum {
    PWE_SEEK_SET = 0,
    PWE_SEEK_CUR = 1,
    PWE_SEEK_END = 2,
};

static inline int pwe_syscall0(unsigned int number) {
    __asm__ volatile ("int $0x80" : "+a"(number) : : "memory");
    int result = (int)number;
    if (result < 0) errno = EIO;
    return result;
}

static inline int pwe_syscall1(unsigned int number, unsigned int arg1) {
    __asm__ volatile ("int $0x80" : "+a"(number) : "b"(arg1) : "memory");
    int result = (int)number;
    if (result < 0) errno = EIO;
    return result;
}

static inline int pwe_syscall3(unsigned int number, unsigned int arg1,
                               unsigned int arg2, unsigned int arg3) {
    __asm__ volatile ("int $0x80" : "+a"(number)
                      : "b"(arg1), "c"(arg2), "d"(arg3)
                      : "memory");
    int result = (int)number;
    if (result < 0) errno = EIO;
    return result;
}

static inline pwe_ssize_t pwe_write(int fd, const void *buffer, pwe_size_t length) {
    return pwe_syscall3(PWE_SYS_FWRITE, (unsigned int)fd, (unsigned int)buffer, length);
}

static inline int pwe_open(const char *path, int flags) {
    return pwe_syscall3(PWE_SYS_OPEN, (unsigned int)path, (unsigned int)flags, 0);
}

static inline pwe_ssize_t pwe_read(int fd, void *buffer, pwe_size_t length) {
    return pwe_syscall3(PWE_SYS_READ, (unsigned int)fd, (unsigned int)buffer, length);
}

static inline int pwe_close(int fd) {
    return pwe_syscall1(PWE_SYS_CLOSE, (unsigned int)fd);
}

static inline int pwe_lseek(int fd, int offset, int whence) {
    return pwe_syscall3(PWE_SYS_LSEEK, (unsigned int)fd,
                        (unsigned int)offset, (unsigned int)whence);
}

static inline int pwe_unlink(const char *path) {
    return pwe_syscall1(PWE_SYS_UNLINK, (unsigned int)path);
}

static inline char *pwe_getcwd(char *buffer, pwe_size_t length) {
    unsigned int address = (unsigned int)buffer;
    int result = pwe_syscall3(PWE_SYS_GETCWD, address, length, 0);
    return result < 0 ? (char *)0 : (char *)address;
}

static inline int pwe_gettimeofday(void *timeval, void *timezone) {
    return pwe_syscall3(PWE_SYS_GETTIMEOFDAY, (unsigned int)timeval,
                        (unsigned int)timezone, 0);
}

static inline int pwe_console_clear(void) {
    return pwe_syscall1(PWE_SYS_CONSOLE, PWE_CONSOLE_CLEAR);
}

static inline int pwe_console_set_cursor(unsigned int row, unsigned int column) {
    return pwe_syscall3(PWE_SYS_CONSOLE, PWE_CONSOLE_SET_CURSOR, row, column);
}

static inline unsigned int pwe_brk(unsigned int address) {
    return (unsigned int)pwe_syscall1(PWE_SYS_BRK, address);
}

static inline void pwe_exit(int status) __attribute__((noreturn));
static inline void pwe_exit(int status) {
    pwe_syscall1(PWE_SYS_EXIT, (unsigned int)status);
    for (;;) __asm__ volatile ("hlt");
}

#endif
