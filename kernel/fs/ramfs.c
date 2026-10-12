#include "fs/ramfs.h"
#include "mm/heap.h"
#include "lib/string.h"
#include "lib/printf.h"

static ramfs_node_t *ramfs_head = NULL;
static size_t ramfs_file_count = 0;

static ramfs_node_t *ramfs_find(const char *name) {
    ramfs_node_t *curr = ramfs_head;
    while (curr) {
        if (strcmp(curr->name, name) == 0) {
            return curr;
        }
        curr = curr->next;
    }
    return NULL;
}

bool ramfs_create(const char *name, const char *initial_data) {
    if (!name || strlen(name) == 0 || strlen(name) >= RAMFS_MAX_NAME) {
        return false;
    }

    if (ramfs_find(name) != NULL) {
        return false; // Already exists
    }

    ramfs_node_t *node = (ramfs_node_t *)kmalloc(sizeof(ramfs_node_t));
    if (!node) return false;

    strncpy(node->name, name, RAMFS_MAX_NAME - 1);
    node->name[RAMFS_MAX_NAME - 1] = '\0';

    if (initial_data) {
        size_t len = strlen(initial_data);
        node->capacity = len + 16;
        node->data = (char *)kmalloc(node->capacity);
        if (!node->data) {
            kfree(node);
            return false;
        }
        memcpy(node->data, initial_data, len);
        node->data[len] = '\0';
        node->size = len;
    } else {
        node->capacity = 32;
        node->data = (char *)kmalloc(node->capacity);
        if (!node->data) {
            kfree(node);
            return false;
        }
        node->data[0] = '\0';
        node->size = 0;
    }

    node->next = ramfs_head;
    ramfs_head = node;
    ramfs_file_count++;
    return true;
}

const char *ramfs_read(const char *name, size_t *out_size) {
    ramfs_node_t *node = ramfs_find(name);
    if (!node) return NULL;
    if (out_size) *out_size = node->size;
    return node->data;
}

bool ramfs_write(const char *name, const char *data, bool append) {
    if (!name || !data) return false;
    ramfs_node_t *node = ramfs_find(name);

    if (!node) {
        return ramfs_create(name, data);
    }

    size_t data_len = strlen(data);
    size_t new_size = append ? (node->size + data_len) : data_len;

    if (new_size + 1 > node->capacity) {
        size_t new_cap = new_size + 32;
        char *new_buf = (char *)krealloc(node->data, new_cap);
        if (!new_buf) return false;
        node->data = new_buf;
        node->capacity = new_cap;
    }

    if (append) {
        memcpy(node->data + node->size, data, data_len);
    } else {
        memcpy(node->data, data, data_len);
    }

    node->size = new_size;
    node->data[node->size] = '\0';
    return true;
}

bool ramfs_delete(const char *name) {
    if (!name) return false;

    ramfs_node_t *curr = ramfs_head;
    ramfs_node_t *prev = NULL;

    while (curr) {
        if (strcmp(curr->name, name) == 0) {
            if (prev) {
                prev->next = curr->next;
            } else {
                ramfs_head = curr->next;
            }
            if (curr->data) {
                kfree(curr->data);
            }
            kfree(curr);
            ramfs_file_count--;
            return true;
        }
        prev = curr;
        curr = curr->next;
    }
    return false;
}

void ramfs_list(void (*callback)(const char *name, size_t size)) {
    if (!callback) return;
    ramfs_node_t *curr = ramfs_head;
    while (curr) {
        callback(curr->name, curr->size);
        curr = curr->next;
    }
}

size_t ramfs_count(void) {
    return ramfs_file_count;
}

void ramfs_init(void) {
    ramfs_head = NULL;
    ramfs_file_count = 0;

    // Prepopulate default system files
    ramfs_create("motd", "Welcome to PweOS!\nA lightweight 32-bit x86 educational operating system.\n");
    ramfs_create("os-release", "NAME=PweOS\nVERSION=0.1.0\nID=pweos\nARCH=i686\n");
    ramfs_create("readme.txt", "PweOS includes heap memory, RTC, CPUID, RAMFS, and shell commands!\nType 'help' to see all commands.\n");
}
