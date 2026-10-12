#ifndef DRIVERS_KEYBOARD_H
#define DRIVERS_KEYBOARD_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#define KEY_UP    ((char)0x81)
#define KEY_DOWN  ((char)0x82)
#define KEY_LEFT  ((char)0x83)
#define KEY_RIGHT ((char)0x84)

void keyboard_init(void);
bool keyboard_has_char(void);
char keyboard_getchar(void);
size_t keyboard_readline(char *buffer, size_t max_len);

#endif // DRIVERS_KEYBOARD_H
