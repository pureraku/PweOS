#include "pweos_libc.h"

#define HEAP_ALIGNMENT 8u
#define HEAP_HEADER_SIZE ((pwe_size_t)sizeof(heap_block_t))

typedef struct heap_block {
    pwe_size_t size;
    struct heap_block *next;
    struct heap_block *previous;
    unsigned int free;
} heap_block_t;

static heap_block_t *heap_first;
static heap_block_t *heap_last;

static pwe_size_t align_size(pwe_size_t size) {
    if (size > (pwe_size_t)-1 - (HEAP_ALIGNMENT - 1)) return 0;
    return (size + HEAP_ALIGNMENT - 1) & ~(HEAP_ALIGNMENT - 1);
}

static heap_block_t *extend_heap(pwe_size_t size) {
    pwe_size_t current = pwe_brk(0);
    if (current == (pwe_size_t)-1) return (heap_block_t *)0;
    if (heap_last && heap_last->free) {
        pwe_size_t tail_end = (pwe_size_t)heap_last + HEAP_HEADER_SIZE + heap_last->size;
        if (tail_end == current && size > heap_last->size) {
            pwe_size_t extra = size - heap_last->size;
            if (extra > (pwe_size_t)-1 - current || pwe_brk(current + extra) == (pwe_size_t)-1)
                return (heap_block_t *)0;
            heap_last->size += extra;
            heap_last->free = 0;
            return heap_last;
        }
    }
    pwe_size_t aligned = align_size(current);
    if (aligned == 0 || aligned > current) {
        if (aligned == 0 || pwe_brk(aligned) == (pwe_size_t)-1) return (heap_block_t *)0;
    }
    if (aligned > (pwe_size_t)-1 - HEAP_HEADER_SIZE
        || size > (pwe_size_t)-1 - aligned - HEAP_HEADER_SIZE) return (heap_block_t *)0;
    pwe_size_t new_break = aligned + HEAP_HEADER_SIZE + size;
    if (pwe_brk(new_break) == (pwe_size_t)-1) return (heap_block_t *)0;

    heap_block_t *block = (heap_block_t *)aligned;
    block->size = size;
    block->next = (heap_block_t *)0;
    block->previous = heap_last;
    block->free = 0;
    if (heap_last) heap_last->next = block;
    else heap_first = block;
    heap_last = block;
    return block;
}

static void split_block(heap_block_t *block, pwe_size_t size) {
    if (block->size < size || block->size - size < HEAP_HEADER_SIZE + HEAP_ALIGNMENT) return;
    heap_block_t *split = (heap_block_t *)((unsigned char *)(block + 1) + size);
    split->size = block->size - size - HEAP_HEADER_SIZE;
    split->next = block->next;
    split->previous = block;
    split->free = 1;
    if (split->next) split->next->previous = split;
    else heap_last = split;
    block->next = split;
    block->size = size;
}

void *malloc(pwe_size_t size) {
    if (size == 0) size = 1;
    size = align_size(size);
    if (size == 0) return (void *)0;

    for (heap_block_t *block = heap_first; block; block = block->next) {
        if (block->free && block->size >= size) {
            split_block(block, size);
            block->free = 0;
            return block + 1;
        }
    }
    heap_block_t *block = extend_heap(size);
    return block ? block + 1 : (void *)0;
}

void free(void *pointer) {
    if (!pointer) return;
    heap_block_t *block = (heap_block_t *)pointer - 1;
    block->free = 1;
    if (block->next && block->next->free) {
        heap_block_t *next = block->next;
        block->size += HEAP_HEADER_SIZE + next->size;
        block->next = next->next;
        if (block->next) block->next->previous = block;
        else heap_last = block;
    }
    if (block->previous && block->previous->free) {
        heap_block_t *previous = block->previous;
        previous->size += HEAP_HEADER_SIZE + block->size;
        previous->next = block->next;
        if (previous->next) previous->next->previous = previous;
        else heap_last = previous;
    }
}

void *calloc(pwe_size_t count, pwe_size_t size) {
    if (size != 0 && count > (pwe_size_t)-1 / size) return (void *)0;
    pwe_size_t total = count * size;
    void *pointer = malloc(total);
    if (pointer) memset(pointer, 0, total);
    return pointer;
}

void *realloc(void *pointer, pwe_size_t size) {
    if (!pointer) return malloc(size);
    if (size == 0) {
        free(pointer);
        return (void *)0;
    }
    pwe_size_t aligned = align_size(size);
    if (aligned == 0) return (void *)0;
    heap_block_t *block = (heap_block_t *)pointer - 1;
    if (block->size >= aligned) {
        split_block(block, aligned);
        return pointer;
    }
    if (block->next && block->next->free
        && block->size + HEAP_HEADER_SIZE + block->next->size >= aligned) {
        heap_block_t *next = block->next;
        block->size += HEAP_HEADER_SIZE + next->size;
        block->next = next->next;
        if (block->next) block->next->previous = block;
        else heap_last = block;
        split_block(block, aligned);
        return pointer;
    }
    void *replacement = malloc(size);
    if (!replacement) return (void *)0;
    memcpy(replacement, pointer, block->size);
    free(pointer);
    return replacement;
}

void *memcpy(void *destination, const void *source, pwe_size_t length) {
    unsigned char *out = (unsigned char *)destination;
    const unsigned char *in = (const unsigned char *)source;
    for (pwe_size_t index = 0; index < length; index++) out[index] = in[index];
    return destination;
}

void *memmove(void *destination, const void *source, pwe_size_t length) {
    unsigned char *out = (unsigned char *)destination;
    const unsigned char *in = (const unsigned char *)source;
    if ((unsigned int)out <= (unsigned int)in) {
        for (pwe_size_t index = 0; index < length; index++) out[index] = in[index];
    } else {
        while (length != 0) {
            length--;
            out[length] = in[length];
        }
    }
    return destination;
}

void *memset(void *destination, int value, pwe_size_t length) {
    unsigned char *out = (unsigned char *)destination;
    for (pwe_size_t index = 0; index < length; index++) out[index] = (unsigned char)value;
    return destination;
}

int memcmp(const void *left, const void *right, pwe_size_t length) {
    const unsigned char *a = (const unsigned char *)left;
    const unsigned char *b = (const unsigned char *)right;
    for (pwe_size_t index = 0; index < length; index++) {
        if (a[index] != b[index]) return (int)a[index] - (int)b[index];
    }
    return 0;
}

void *memchr(const void *memory, int character, pwe_size_t length) {
    const unsigned char *bytes = (const unsigned char *)memory;
    unsigned char target = (unsigned char)character;
    for (pwe_size_t index = 0; index < length; index++) {
        if (bytes[index] == target) return (void *)(bytes + index);
    }
    return (void *)0;
}

pwe_size_t strlen(const char *text) {
    pwe_size_t length = 0;
    while (text[length]) length++;
    return length;
}

pwe_size_t strnlen(const char *text, pwe_size_t maximum) {
    pwe_size_t length = 0;
    while (length < maximum && text[length]) length++;
    return length;
}

int strcmp(const char *left, const char *right) {
    while (*left && *left == *right) { left++; right++; }
    return (unsigned char)*left - (unsigned char)*right;
}

int strncmp(const char *left, const char *right, pwe_size_t length) {
    while (length && *left && *left == *right) { left++; right++; length--; }
    return length ? (unsigned char)*left - (unsigned char)*right : 0;
}

char *strcpy(char *destination, const char *source) {
    char *result = destination;
    while ((*destination++ = *source++) != '\0') {}
    return result;
}

char *strncpy(char *destination, const char *source, pwe_size_t length) {
    char *result = destination;
    pwe_size_t index = 0;
    while (index < length && source[index]) { destination[index] = source[index]; index++; }
    while (index < length) destination[index++] = '\0';
    return result;
}

char *strcat(char *destination, const char *source) {
    strcpy(destination + strlen(destination), source);
    return destination;
}

char *strchr(const char *text, int character) {
    char target = (char)character;
    do {
        if (*text == target) return (char *)text;
    } while (*text++);
    return (char *)0;
}

char *strpbrk(const char *text, const char *accept) {
    for (; *text; text++) {
        for (const char *candidate = accept; *candidate; candidate++) {
            if (*text == *candidate) return (char *)text;
        }
    }
    return (char *)0;
}

char *strrchr(const char *text, int character) {
    char target = (char)character;
    const char *last = (const char *)0;
    do {
        if (*text == target) last = text;
    } while (*text++);
    return (char *)last;
}

char *strstr(const char *text, const char *needle) {
    if (!*needle) return (char *)text;
    for (; *text; text++) {
        const char *left = text;
        const char *right = needle;
        while (*left && *right && *left == *right) { left++; right++; }
        if (!*right) return (char *)text;
    }
    return (char *)0;
}

char *strdup(const char *text) {
    pwe_size_t length = strlen(text) + 1;
    char *copy = (char *)malloc(length);
    return copy ? memcpy(copy, text, length) : (char *)0;
}
