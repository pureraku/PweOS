#ifndef PWEOS_H
#define PWEOS_H

typedef unsigned int pwe_size_t;
typedef int pwe_ssize_t;

enum {
    PWE_SYS_EXIT = 0,
    PWE_SYS_WRITE = 1,
    PWE_SYS_OPEN = 2,
    PWE_SYS_READ = 3,
    PWE_SYS_CLOSE = 4,
    PWE_SYS_BRK = 5,
};

static inline int pwe_syscall0(unsigned int number) {
    __asm__ volatile ("int $0x80" : "+a"(number) : : "memory");
    return (int)number;
}

static inline int pwe_syscall1(unsigned int number, unsigned int arg1) {
    __asm__ volatile ("int $0x80" : "+a"(number) : "b"(arg1) : "memory");
    return (int)number;
}

static inline int pwe_syscall3(unsigned int number, unsigned int arg1,
                               unsigned int arg2, unsigned int arg3) {
    __asm__ volatile ("int $0x80" : "+a"(number)
                      : "b"(arg1), "c"(arg2), "d"(arg3)
                      : "memory");
    return (int)number;
}

static inline pwe_ssize_t pwe_write(const void *buffer, pwe_size_t length) {
    return pwe_syscall3(PWE_SYS_WRITE, (unsigned int)buffer, length, 0);
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

static inline unsigned int pwe_brk(unsigned int address) {
    return (unsigned int)pwe_syscall1(PWE_SYS_BRK, address);
}

static inline void pwe_exit(int status) __attribute__((noreturn));
static inline void pwe_exit(int status) {
    pwe_syscall1(PWE_SYS_EXIT, (unsigned int)status);
    for (;;) __asm__ volatile ("hlt");
}

#endif
