#include "fcntl.h"
#include "errno.h"
#include "stdio.h"
#include "stdlib.h"
#include "string.h"
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

char *realpath(const char *path, char *resolved_path) {
    char cwd[4096];
    char *absolute;
    char *canonical;
    size_t cwd_length;
    size_t path_length;
    size_t absolute_length;
    size_t output_length = 1;
    size_t position;
    FILE *file;

    if (!path || !*path) {
        errno = EINVAL;
        return NULL;
    }
    if (!getcwd(cwd, sizeof(cwd))) {
        errno = ENOENT;
        return NULL;
    }

    cwd_length = strlen(cwd);
    path_length = strlen(path);
    if (path_length > (size_t)-1 - cwd_length - 2) {
        errno = ENOMEM;
        return NULL;
    }
    absolute = (char *)malloc(cwd_length + path_length + 2);
    if (!absolute) {
        errno = ENOMEM;
        return NULL;
    }
    if (path[0] == '/') {
        strcpy(absolute, path);
    } else {
        strcpy(absolute, cwd);
        absolute_length = cwd_length;
        if (absolute_length == 0 || absolute[absolute_length - 1] != '/')
            absolute[absolute_length++] = '/';
        strcpy(absolute + absolute_length, path);
    }

    absolute_length = strlen(absolute);
    canonical = (char *)malloc(absolute_length + 2);
    if (!canonical) {
        free(absolute);
        errno = ENOMEM;
        return NULL;
    }
    canonical[0] = '/';
    canonical[1] = '\0';

    position = 0;
    while (position < absolute_length) {
        size_t start;
        size_t length;

        while (position < absolute_length && absolute[position] == '/')
            position++;
        start = position;
        while (position < absolute_length && absolute[position] != '/')
            position++;
        length = position - start;

        if (length == 0 || (length == 1 && absolute[start] == '.'))
            continue;
        if (length == 2 && absolute[start] == '.' && absolute[start + 1] == '.') {
            if (output_length > 1) {
                while (output_length > 1 && canonical[output_length - 1] != '/')
                    output_length--;
                canonical[output_length] = '\0';
            }
            continue;
        }

        if (output_length > 1)
            canonical[output_length++] = '/';
        memcpy(canonical + output_length, absolute + start, length);
        output_length += length;
        canonical[output_length] = '\0';
    }

    free(absolute);
    file = fopen(canonical, "rb");
    if (!file) {
        free(canonical);
        errno = ENOENT;
        return NULL;
    }
    fclose(file);

    if (resolved_path) {
        strcpy(resolved_path, canonical);
        free(canonical);
        return resolved_path;
    }
    return canonical;
}
