#include "errno.h"

int errno;

char *strerror(int error) {
    switch (error) {
    case 0: return "success";
    case EPERM: return "operation not permitted";
    case ENOENT: return "no such file or directory";
    case EINTR: return "interrupted system call";
    case EIO: return "input/output error";
    case EBADF: return "bad file descriptor";
    case EAGAIN: return "resource temporarily unavailable";
    case ENOMEM: return "out of memory";
    case EACCES: return "permission denied";
    case EFAULT: return "bad address";
    case EEXIST: return "file exists";
    case ENOTDIR: return "not a directory";
    case EISDIR: return "is a directory";
    case EINVAL: return "invalid argument";
    case EMFILE: return "too many open files";
    case ENOSPC: return "no space left";
    case ESPIPE: return "invalid seek";
    case EROFS: return "read-only file system";
    case EPIPE: return "broken pipe";
    case ERANGE: return "result out of range";
    default: return "unknown error";
    }
}
