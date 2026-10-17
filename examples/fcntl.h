#ifndef PWEOS_FCNTL_H
#define PWEOS_FCNTL_H

#include "pweos.h"

#define O_RDONLY 0
#define O_WRONLY 1
#define O_RDWR 2
#define O_CREAT 0x0040
#define O_TRUNC 0x0200
#define O_APPEND 0x0400
#define O_BINARY 0

int open(const char *path, int flags, ...);

#endif
