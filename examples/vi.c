#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "pweos.h"

#define VI_MAX_BYTES 65536u
#define SCREEN_WIDTH 80u
#define SCREEN_HEIGHT 25u
#define KEY_UP 0x81
#define KEY_DOWN 0x82
#define KEY_LEFT 0x83
#define KEY_RIGHT 0x84
#define KEY_DELETE 0x85

static void line_bounds(const char *text, size_t length, size_t wanted,
                        size_t *start, size_t *end) {
    size_t line = 0;
    size_t line_start = 0;
    for (size_t i = 0; i < length; i++) {
        if (text[i] == '\n') {
            if (line == wanted) {
                *start = line_start;
                *end = i;
                return;
            }
            line++;
            line_start = i + 1;
        }
    }
    if (line == wanted) {
        *start = line_start;
        *end = length;
    } else {
        *start = length;
        *end = length;
    }
}

static void cursor_position(const char *text, size_t cursor,
                            size_t *line, size_t *column) {
    *line = 0;
    *column = 0;
    for (size_t i = 0; i < cursor; i++) {
        if (text[i] == '\n') {
            (*line)++;
            *column = 0;
        } else {
            (*column)++;
        }
    }
}

static size_t clamp_cursor(const char *text, size_t length, size_t cursor) {
    size_t line, column, start, end;
    if (cursor > length) cursor = length;
    cursor_position(text, cursor, &line, &column);
    line_bounds(text, length, line, &start, &end);
    (void)column;
    return end > start && cursor >= end ? end - 1 : cursor;
}

static void draw(const char *path, const char *text, size_t length,
                 size_t cursor, int insert_mode, int pending_delete,
                 const char *status, const char *command) {
    const size_t header_rows = 3;
    const size_t content_rows = SCREEN_HEIGHT - header_rows - 1;
    size_t line, column;
    cursor_position(text, cursor, &line, &column);
    size_t first_line = line > content_rows - 1 ? line - (content_rows - 1) : 0;
    size_t horizontal_offset = column > SCREEN_WIDTH - 2 ? column - (SCREEN_WIDTH - 2) : 0;

    pwe_console_clear();
    printf("PweOS vi - %s\n", path);
    if (command) printf("COMMAND\n");
    else if (insert_mode) printf("-- INSERT --\n");
    else if (pending_delete) printf("NORMAL -- press d again to delete the line\n");
    else printf("NORMAL -- h/j/k/l move | i insert | :w save | :q quit\n");
    printf("----------------------------------------------------\n");

    for (size_t row = 0; row < content_rows; row++) {
        size_t start, end;
        line_bounds(text, length, first_line + row, &start, &end);
        size_t skip = first_line + row == line ? horizontal_offset : 0;
        for (size_t i = start + skip; i < end && i < start + skip + SCREEN_WIDTH - 1; i++) {
            unsigned char byte = (unsigned char)text[i];
            putchar(byte >= 32 && byte <= 126 ? byte : '.');
        }
        if (row + 1 < content_rows) putchar('\n');
    }

    pwe_console_set_cursor(SCREEN_HEIGHT - 1, 0);
    if (command) printf(":%s", command);
    else printf("%s", status);
    size_t screen_row = header_rows + line - first_line;
    size_t screen_column = column > horizontal_offset ? column - horizontal_offset : 0;
    pwe_console_set_cursor((unsigned int)screen_row, (unsigned int)screen_column);
}

static int save_file(const char *path, const char *text, size_t length) {
    FILE *file = fopen(path, "wb");
    if (!file) return 0;
    size_t written = fwrite(text, 1, length, file);
    int closed = fclose(file);
    return written == length && closed == 0;
}

static int read_command(char *command, size_t capacity,
                        const char *path, const char *text, size_t length,
                        size_t cursor) {
    size_t count = 0;
    command[0] = '\0';
    for (;;) {
        draw(path, text, length, cursor, 0, 0, "", command);
        int key = getchar();
        if (key == EOF) return 0;
        if (key == 27) return 0;
        if (key == '\n' || key == '\r') {
            command[count] = '\0';
            return 1;
        }
        if (key == '\b') {
            if (count) command[--count] = '\0';
        } else if (key >= 32 && key <= 126 && count + 1 < capacity) {
            command[count++] = (char)key;
            command[count] = '\0';
        }
    }
}

int main(int argc, char **argv) {
    if (argc < 2) {
        printf("Usage: vi <path>\n");
        return 1;
    }

    const char *path = argv[1];
    char *text = malloc(VI_MAX_BYTES + 1);
    if (!text) {
        printf("vi: not enough memory\n");
        return 1;
    }

    size_t length = 0;
    FILE *input = fopen(path, "rb");
    if (input) {
        length = fread(text, 1, VI_MAX_BYTES + 1, input);
        fclose(input);
        if (length > VI_MAX_BYTES) {
            printf("vi: file exceeds the 64 KiB editor limit\n");
            free(text);
            return 1;
        }
    }
    text[length] = '\0';

    size_t cursor = 0;
    size_t preferred_column = 0;
    int insert_mode = 0;
    int insert_changed = 0;
    int pending_delete = 0;
    int dirty = 0;
    const char *message = "NORMAL";

    for (;;) {
        draw(path, text, length, cursor, insert_mode, pending_delete, message, NULL);
        int key = getchar();
        if (key == EOF) {
            pwe_console_clear();
            free(text);
            return 1;
        }

        if (insert_mode) {
            if (key == 27) {
                insert_mode = 0;
                if (insert_changed && cursor) cursor--;
                insert_changed = 0;
                cursor = clamp_cursor(text, length, cursor);
                message = "NORMAL";
            } else if (key == KEY_LEFT && cursor) {
                cursor--;
            } else if (key == KEY_RIGHT && cursor < length) {
                cursor++;
            } else if (key == KEY_UP || key == KEY_DOWN) {
                size_t line, column, start, end, target_start, target_end;
                cursor_position(text, cursor, &line, &column);
                line_bounds(text, length, line, &start, &end);
                int has_target = key == KEY_UP ? line > 0 : end < length;
                size_t target_line = key == KEY_UP && line ? line - 1 : line + 1;
                line_bounds(text, length, target_line, &target_start, &target_end);
                if (has_target) {
                    cursor = target_start + (column < target_end - target_start ? column : target_end - target_start);
                    preferred_column = column;
                }
                (void)start;
                (void)end;
            } else if (key == '\b') {
                if (cursor) {
                    memmove(text + cursor - 1, text + cursor, length - cursor + 1);
                    cursor--;
                    length--;
                    dirty = insert_changed = 1;
                }
            } else if (key == KEY_DELETE) {
                if (cursor < length) {
                    memmove(text + cursor, text + cursor + 1, length - cursor);
                    length--;
                    dirty = insert_changed = 1;
                }
            } else if (key == '\n' || key == '\r') {
                if (length < VI_MAX_BYTES) {
                    memmove(text + cursor + 1, text + cursor, length - cursor + 1);
                    text[cursor++] = '\n';
                    length++;
                    dirty = insert_changed = 1;
                    preferred_column = 0;
                }
            } else if (key == '\t') {
                if (length + 4 <= VI_MAX_BYTES) {
                    memmove(text + cursor + 4, text + cursor, length - cursor + 1);
                    memset(text + cursor, ' ', 4);
                    cursor += 4;
                    length += 4;
                    dirty = insert_changed = 1;
                }
            } else if (key >= 32 && key <= 126 && length < VI_MAX_BYTES) {
                memmove(text + cursor + 1, text + cursor, length - cursor + 1);
                text[cursor++] = (char)key;
                length++;
                dirty = insert_changed = 1;
            }
            continue;
        }

        if (pending_delete && key == 'd') {
            size_t line, column, start, end;
            cursor_position(text, cursor, &line, &column);
            line_bounds(text, length, line, &start, &end);
            size_t remove_start = end == length && start ? start - 1 : start;
            size_t remove_end = end < length ? end + 1 : end;
            if (remove_end > remove_start) {
                memmove(text + remove_start, text + remove_end, length - remove_end + 1);
                length -= remove_end - remove_start;
                cursor = remove_start;
                dirty = 1;
            }
            pending_delete = 0;
            cursor = clamp_cursor(text, length, cursor);
            message = "NORMAL";
            (void)column;
            continue;
        }
        if (pending_delete) message = "NORMAL";
        pending_delete = 0;

        if (key == ':') {
            char command[33];
            if (!read_command(command, sizeof(command), path, text, length, cursor)) {
                message = "NORMAL";
            } else if (!strcmp(command, "w")) {
                if (save_file(path, text, length)) {
                    dirty = 0;
                    message = "written";
                } else message = "write failed";
            } else if (!strcmp(command, "q") && dirty) {
                message = "unsaved changes; use :q! to discard";
            } else if (!strcmp(command, "q") || !strcmp(command, "q!")) {
                pwe_console_clear();
                free(text);
                return 0;
            } else if (!strcmp(command, "wq")) {
                if (save_file(path, text, length)) {
                    pwe_console_clear();
                    free(text);
                    return 0;
                }
                message = "write failed";
            } else message = "unknown command";
            continue;
        }

        if (key == 'd') {
            pending_delete = 1;
            message = "press d again to delete the line";
        } else if (key == 'i') {
            insert_mode = 1;
            insert_changed = 0;
        } else if (key == 'a' || key == 'A') {
            size_t line, column, start, end;
            cursor_position(text, cursor, &line, &column);
            line_bounds(text, length, line, &start, &end);
            cursor = key == 'A' ? end : (cursor < end ? cursor + 1 : cursor);
            insert_mode = 1;
            insert_changed = 0;
            (void)column;
            (void)start;
        } else if ((key == 'o' || key == 'O') && length < VI_MAX_BYTES) {
            size_t line, column, start, end;
            cursor_position(text, cursor, &line, &column);
            line_bounds(text, length, line, &start, &end);
            size_t at = key == 'o' ? end : start;
            memmove(text + at + 1, text + at, length - at + 1);
            text[at] = '\n';
            length++;
            cursor = key == 'o' ? at + 1 : at;
            dirty = 1;
            insert_mode = 1;
            insert_changed = 0;
            preferred_column = 0;
            (void)column;
        } else if (key == 'x') {
            if (cursor < length && text[cursor] != '\n') {
                memmove(text + cursor, text + cursor + 1, length - cursor);
                length--;
                dirty = 1;
                cursor = clamp_cursor(text, length, cursor);
            }
        } else if (key == '0' || key == '$') {
            size_t line, column, start, end;
            cursor_position(text, cursor, &line, &column);
            line_bounds(text, length, line, &start, &end);
            cursor = key == '0' ? start : (end > start ? end - 1 : end);
            preferred_column = cursor - start;
            (void)column;
        } else if (key == 'h' || key == KEY_LEFT) {
            size_t line, column, start, end;
            cursor_position(text, cursor, &line, &column);
            line_bounds(text, length, line, &start, &end);
            if (cursor > start) cursor--;
            preferred_column = cursor - start;
            (void)end;
            (void)column;
        } else if (key == 'l' || key == KEY_RIGHT) {
            size_t line, column, start, end;
            cursor_position(text, cursor, &line, &column);
            line_bounds(text, length, line, &start, &end);
            if (cursor + 1 < end) cursor++;
            preferred_column = cursor - start;
            (void)column;
        } else if (key == 'j' || key == 'k' || key == KEY_UP || key == KEY_DOWN) {
            size_t line, column, start, end, target_start, target_end;
            cursor_position(text, cursor, &line, &column);
            line_bounds(text, length, line, &start, &end);
            int upward = key == 'k' || key == KEY_UP;
            int has_target = upward ? line > 0 : end < length;
            size_t target_line = upward && line ? line - 1 : line + 1;
            line_bounds(text, length, target_line, &target_start, &target_end);
            if (has_target) {
                cursor = target_start + (preferred_column < target_end - target_start ? preferred_column : target_end - target_start);
            }
            (void)column;
            (void)start;
            (void)end;
        }
    }
}
