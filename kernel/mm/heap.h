#ifndef MM_HEAP_H
#define MM_HEAP_H

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

#define HEAP_START_ADDR 0x100000  // 1 MB mark
#define HEAP_INITIAL_SIZE (2 * 1024 * 1024) // 2 MB heap

typedef struct {
    size_t total_bytes;
    size_t used_bytes;
    size_t free_bytes;
    size_t num_allocations;
    size_t num_blocks;
} heap_stats_t;

void heap_init(uint32_t start, size_t size);
void *kmalloc(size_t size);
void *kcalloc(size_t num, size_t size);
void *krealloc(void *ptr, size_t new_size);
void kfree(void *ptr);
void heap_get_stats(heap_stats_t *stats);

#endif // MM_HEAP_H
