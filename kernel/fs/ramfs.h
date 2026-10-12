#ifndef FS_RAMFS_H
#define FS_RAMFS_H

#include <stddef.h>
#include <stdbool.h>

#define RAMFS_MAX_NAME 32

typedef struct ramfs_node {
    char name[RAMFS_MAX_NAME];
    char *data;
    size_t size;
    size_t capacity;
    struct ramfs_node *next;
} ramfs_node_t;

void ramfs_init(void);
bool ramfs_create(const char *name, const char *initial_data);
const char *ramfs_read(const char *name, size_t *out_size);
bool ramfs_write(const char *name, const char *data, bool append);
bool ramfs_delete(const char *name);
void ramfs_list(void (*callback)(const char *name, size_t size));
size_t ramfs_count(void);

#endif // FS_RAMFS_H
