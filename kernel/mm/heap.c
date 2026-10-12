#include "mm/heap.h"
#include "lib/string.h"
#include "lib/printf.h"

#define HEAP_MAGIC 0xCAFEBABE
#define ALIGN8(x) (((x) + 7) & ~7)

typedef struct heap_block {
    uint32_t magic;
    size_t size;
    bool is_free;
    struct heap_block *next;
    struct heap_block *prev;
} heap_block_t;

#define BLOCK_HEADER_SIZE (sizeof(heap_block_t))

static heap_block_t *heap_head = NULL;
static size_t heap_total_size = 0;

void heap_init(uint32_t start, size_t size) {
    heap_total_size = size;
    heap_head = (heap_block_t *)start;
    heap_head->magic = HEAP_MAGIC;
    heap_head->size = size - BLOCK_HEADER_SIZE;
    heap_head->is_free = true;
    heap_head->next = NULL;
    heap_head->prev = NULL;
}

void *kmalloc(size_t size) {
    if (size == 0 || !heap_head) return NULL;

    size_t aligned_size = ALIGN8(size);
    heap_block_t *curr = heap_head;

    // First-fit search
    while (curr) {
        if (curr->magic != HEAP_MAGIC) {
            printf("[HEAP ERROR] Corrupted heap block detected at %p!\n", curr);
            return NULL;
        }

        if (curr->is_free && curr->size >= aligned_size) {
            // Check if block can be split
            if (curr->size >= aligned_size + BLOCK_HEADER_SIZE + 16) {
                heap_block_t *split_block = (heap_block_t *)((uint8_t *)curr + BLOCK_HEADER_SIZE + aligned_size);
                split_block->magic = HEAP_MAGIC;
                split_block->size = curr->size - aligned_size - BLOCK_HEADER_SIZE;
                split_block->is_free = true;
                split_block->next = curr->next;
                split_block->prev = curr;

                if (curr->next) {
                    curr->next->prev = split_block;
                }
                curr->next = split_block;
                curr->size = aligned_size;
            }

            curr->is_free = false;
            return (void *)((uint8_t *)curr + BLOCK_HEADER_SIZE);
        }

        curr = curr->next;
    }

    printf("[HEAP WARNING] Out of memory! Requested %u bytes.\n", (unsigned int)size);
    return NULL;
}

void kfree(void *ptr) {
    if (!ptr) return;

    heap_block_t *block = (heap_block_t *)((uint8_t *)ptr - BLOCK_HEADER_SIZE);
    if (block->magic != HEAP_MAGIC) {
        printf("[HEAP ERROR] Invalid or double-free detected at %p!\n", ptr);
        return;
    }

    block->is_free = true;

    // Merge with next block if free (coalescing)
    if (block->next && block->next->is_free) {
        block->size += BLOCK_HEADER_SIZE + block->next->size;
        block->next = block->next->next;
        if (block->next) {
            block->next->prev = block;
        }
    }

    // Merge with previous block if free
    if (block->prev && block->prev->is_free) {
        block->prev->size += BLOCK_HEADER_SIZE + block->size;
        block->prev->next = block->next;
        if (block->next) {
            block->next->prev = block->prev;
        }
    }
}

void *kcalloc(size_t num, size_t size) {
    size_t total = num * size;
    void *ptr = kmalloc(total);
    if (ptr) {
        memset(ptr, 0, total);
    }
    return ptr;
}

void *krealloc(void *ptr, size_t new_size) {
    if (!ptr) return kmalloc(new_size);
    if (new_size == 0) {
        kfree(ptr);
        return NULL;
    }

    heap_block_t *block = (heap_block_t *)((uint8_t *)ptr - BLOCK_HEADER_SIZE);
    if (block->magic != HEAP_MAGIC) {
        return NULL;
    }

    size_t aligned_new = ALIGN8(new_size);
    if (block->size >= aligned_new) {
        return ptr;
    }

    void *new_ptr = kmalloc(new_size);
    if (new_ptr) {
        memcpy(new_ptr, ptr, block->size);
        kfree(ptr);
    }
    return new_ptr;
}

void heap_get_stats(heap_stats_t *stats) {
    if (!stats) return;

    stats->total_bytes = heap_total_size;
    stats->used_bytes = 0;
    stats->free_bytes = 0;
    stats->num_allocations = 0;
    stats->num_blocks = 0;

    heap_block_t *curr = heap_head;
    while (curr) {
        stats->num_blocks++;
        if (curr->is_free) {
            stats->free_bytes += curr->size;
        } else {
            stats->used_bytes += curr->size;
            stats->num_allocations++;
        }
        curr = curr->next;
    }
}
