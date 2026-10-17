#include "fcntl.h"
#include "unistd.h"

int open(const char *path, int flags, ...) {
    return pwe_open(path, flags);
}

ssize_t read(int fd, void *buffer, size_t length) {
    return pwe_read(fd, buffer, (pwe_size_t)length);
}

ssize_t write(int fd, const void *buffer, size_t length) {
    return pwe_write(fd, buffer, (pwe_size_t)length);
}

int close(int fd) {
    return pwe_close(fd);
}

off_t lseek(int fd, off_t offset, int whence) {
    return (off_t)pwe_lseek(fd, (int)offset, whence);
}

int unlink(const char *path) {
    return pwe_unlink(path);
}

char *getcwd(char *buffer, size_t length) {
    return pwe_getcwd(buffer, (pwe_size_t)length);
}
