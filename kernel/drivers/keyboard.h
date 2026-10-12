#ifndef DRIVERS_KEYBOARD_H
#define DRIVERS_KEYBOARD_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

void keyboard_init(void);
bool keyboard_has_char(void);
char keyboard_getchar(void);
size_t keyboard_readline(char *buffer, size_t max_len);

#endif // DRIVERS_KEYBOARD_H
