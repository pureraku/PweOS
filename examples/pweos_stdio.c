#include "stdio.h"
#include "errno.h"
#include "pweos_libc.h"

static FILE standard_input = {0, 1, 0, 0, 0, 0, -1};
static FILE standard_output = {1, 0, 1, 0, 0, 0, -1};
static FILE standard_error = {2, 0, 1, 0, 0, 0, -1};
FILE *stdin = &standard_input;
FILE *stdout = &standard_output;
FILE *stderr = &standard_error;

static int parse_mode(const char *mode, int *readable, int *writable, int *flags) {
    if (!mode || !mode[0]) return 0;
    int plus = 0;
    for (const char *c = mode + 1; *c; c++) {
        if (*c == '+') plus = 1;
        else if (*c != 'b') return 0;
    }
    if (mode[0] == 'r') {
        *readable = 1; *writable = plus; *flags = plus ? PWE_O_RDWR : PWE_O_RDONLY;
    } else if (mode[0] == 'w') {
        *readable = plus; *writable = 1;
        *flags = (plus ? PWE_O_RDWR : PWE_O_WRONLY) | PWE_O_CREAT | PWE_O_TRUNC;
    } else if (mode[0] == 'a') {
        *readable = plus; *writable = 1;
        *flags = (plus ? PWE_O_RDWR : PWE_O_WRONLY) | PWE_O_CREAT | PWE_O_APPEND;
    } else return 0;
    return 1;
}

FILE *fdopen(int fd, const char *mode) {
    int readable, writable, flags;
    if (fd < 0 || !parse_mode(mode, &readable, &writable, &flags)) return (FILE *)0;
    (void)flags;
    FILE *stream = (FILE *)malloc(sizeof(FILE));
    if (!stream) return (FILE *)0;
    stream->fd = fd;
    stream->readable = readable;
    stream->writable = writable;
    stream->owns_fd = fd >= 3;
    stream->eof = 0;
    stream->error = 0;
    stream->pushed_char = -1;
    return stream;
}

FILE *fopen(const char *path, const char *mode) {
    int readable, writable, flags;
    if (!parse_mode(mode, &readable, &writable, &flags)) return (FILE *)0;
    int fd = pwe_open(path, flags);
    if (fd < 0) return (FILE *)0;
    FILE *stream = (FILE *)malloc(sizeof(FILE));
    if (!stream) {
        pwe_close(fd);
        return (FILE *)0;
    }
    stream->fd = fd;
    stream->readable = readable;
    stream->writable = writable;
    stream->owns_fd = 1;
    stream->eof = 0;
    stream->error = 0;
    stream->pushed_char = -1;
    return stream;
}

FILE *freopen(const char *path, const char *mode, FILE *stream) {
    if (!stream) return (FILE *)0;
    int readable, writable, flags;
    if (!parse_mode(mode, &readable, &writable, &flags)) return (FILE *)0;
    if (stream->owns_fd) pwe_close(stream->fd);
    int fd = pwe_open(path, flags);
    if (fd < 0) {
        stream->error = 1;
        stream->fd = -1;
        return (FILE *)0;
    }
    stream->fd = fd;
    stream->readable = readable;
    stream->writable = writable;
    stream->owns_fd = 1;
    stream->eof = 0;
    stream->error = 0;
    stream->pushed_char = -1;
    return stream;
}

int fclose(FILE *stream) {
    if (!stream) return EOF;
    int result = fflush(stream);
    if (stream->owns_fd && pwe_close(stream->fd) < 0) result = EOF;
    if (stream != stdin && stream != stdout && stream != stderr) free(stream);
    return result;
}

int fflush(FILE *stream) {
    (void)stream;
    return 0;
}

size_t fread(void *buffer, size_t size, size_t count, FILE *stream) {
    if (!stream || !stream->readable || size == 0 || count == 0) return 0;
    if (count > (size_t)-1 / size) { stream->error = 1; return 0; }
    size_t requested = size * count;
    size_t received = 0;
    unsigned char *out = (unsigned char *)buffer;
    if (stream->pushed_char >= 0 && requested != 0) {
        out[received++] = (unsigned char)stream->pushed_char;
        stream->pushed_char = -1;
    }
    while (received < requested) {
        size_t chunk = requested - received;
        if (chunk > 4096) chunk = 4096;
        int result = pwe_read(stream->fd, out + received, (pwe_size_t)chunk);
        if (result < 0) { stream->error = 1; break; }
        if (result == 0) { stream->eof = 1; break; }
        received += (size_t)result;
    }
    return received / size;
}

size_t fwrite(const void *buffer, size_t size, size_t count, FILE *stream) {
    if (!stream || !stream->writable || size == 0 || count == 0) return 0;
    if (count > (size_t)-1 / size) { stream->error = 1; return 0; }
    size_t requested = size * count;
    size_t written = 0;
    const unsigned char *input = (const unsigned char *)buffer;
    while (written < requested) {
        size_t chunk = requested - written;
        if (chunk > 4096) chunk = 4096;
        int result = pwe_write(stream->fd, input + written, (pwe_size_t)chunk);
        if (result < 0 || result == 0) { stream->error = 1; break; }
        written += (size_t)result;
    }
    return written / size;
}

int fseek(FILE *stream, long offset, int origin) {
    if (!stream || pwe_lseek(stream->fd, (int)offset, origin) < 0) {
        if (stream) stream->error = 1;
        return -1;
    }
    stream->eof = 0;
    stream->pushed_char = -1;
    return 0;
}

long ftell(FILE *stream) {
    if (!stream) return -1;
    int offset = pwe_lseek(stream->fd, 0, PWE_SEEK_CUR);
    if (offset < 0) stream->error = 1;
    return (long)offset;
}

void rewind(FILE *stream) {
    if (fseek(stream, 0, PWE_SEEK_SET) == 0) stream->error = 0;
}

int fgetc(FILE *stream) {
    unsigned char byte;
    if (!stream || !stream->readable) return EOF;
    if (stream->pushed_char >= 0) {
        int result = stream->pushed_char;
        stream->pushed_char = -1;
        stream->eof = 0;
        return result;
    }
    int result = pwe_read(stream->fd, &byte, 1);
    if (result < 0) { stream->error = 1; return EOF; }
    if (result == 0) { stream->eof = 1; return EOF; }
    return byte;
}

int getc(FILE *stream) { return fgetc(stream); }
int getchar(void) { return fgetc(stdin); }

int ungetc(int character, FILE *stream) {
    if (!stream || character == EOF || stream->pushed_char >= 0) return EOF;
    stream->pushed_char = (unsigned char)character;
    stream->eof = 0;
    return stream->pushed_char;
}

char *fgets(char *buffer, int length, FILE *stream) {
    if (!buffer || length <= 0 || !stream) return (char *)0;
    if (length == 1) { buffer[0] = '\0'; return buffer; }
    int index = 0;
    while (index + 1 < length) {
        int character = fgetc(stream);
        if (character == EOF) break;
        buffer[index++] = (char)character;
        if (character == '\n') break;
    }
    if (index == 0) return (char *)0;
    buffer[index] = '\0';
    return buffer;
}

int fputc(int character, FILE *stream) {
    unsigned char byte = (unsigned char)character;
    return fwrite(&byte, 1, 1, stream) == 1 ? byte : EOF;
}

int putchar(int character) { return fputc(character, stdout); }

int fputs(const char *text, FILE *stream) {
    size_t length = strlen(text);
    return fwrite(text, 1, length, stream) == length ? 0 : EOF;
}

int puts(const char *text) {
    if (fputs(text, stdout) == EOF || fputc('\n', stdout) == EOF) return EOF;
    return 0;
}

int feof(FILE *stream) { return stream ? stream->eof : 0; }
int ferror(FILE *stream) { return stream ? stream->error : 0; }
void clearerr(FILE *stream) { if (stream) { stream->eof = 0; stream->error = 0; } }

int remove(const char *path) { return pwe_unlink(path); }

void perror(const char *prefix) {
    if (prefix && *prefix) { fputs(prefix, stderr); fputs(": ", stderr); }
    fputs(strerror(errno), stderr);
    fputc('\n', stderr);
}

typedef struct format_state {
    FILE *stream;
    char *buffer;
    size_t capacity;
    size_t count;
    int error;
    char chunk[128];
    size_t chunk_length;
} format_state_t;

static void format_emit(format_state_t *state, char character) {
    if (state->buffer) {
        if (state->capacity && state->count + 1 < state->capacity)
            state->buffer[state->count] = character;
    } else {
        state->chunk[state->chunk_length++] = character;
        if (state->chunk_length == sizeof(state->chunk)) {
            if (fwrite(state->chunk, 1, state->chunk_length, state->stream) != state->chunk_length)
                state->error = 1;
            state->chunk_length = 0;
        }
    }
    state->count++;
}

static void format_repeat(format_state_t *state, char character, size_t count) {
    while (count--) format_emit(state, character);
}

static void format_string(format_state_t *state, const char *text, int width, int precision, int left) {
    size_t length = strlen(text);
    if (precision >= 0 && length > (size_t)precision) length = (size_t)precision;
    if (!left && width > 0 && (size_t)width > length) format_repeat(state, ' ', (size_t)width - length);
    for (size_t index = 0; index < length; index++) format_emit(state, text[index]);
    if (left && width > 0 && (size_t)width > length) format_repeat(state, ' ', (size_t)width - length);
}

static void format_number(format_state_t *state, unsigned long long value, int negative,
                          unsigned int base, int uppercase, int width, int precision,
                          int left, int zero, int alternate, int plus, int space, int pointer) {
    char digits[32];
    size_t length = 0;
    const char *alphabet = uppercase ? "0123456789ABCDEF" : "0123456789abcdef";
    if (value == 0) {
        if (precision != 0) digits[length++] = '0';
    } else {
        while (value) { digits[length++] = alphabet[value % base]; value /= base; }
    }
    char sign = negative ? '-' : (plus ? '+' : (space ? ' ' : 0));
    size_t prefix_length = pointer || (alternate && base == 16 && length != 0) ? 2 :
                           alternate && base == 8 ? 1 : 0;
    size_t zeros = precision > 0 && (size_t)precision > length ? (size_t)precision - length : 0;
    size_t total = length + zeros + (sign != 0) + prefix_length;
    size_t padding = width > 0 && (size_t)width > total ? (size_t)width - total : 0;
    if (zero && !left && precision < 0) { zeros += padding; padding = 0; }
    if (!left) format_repeat(state, ' ', padding);
    if (sign) format_emit(state, sign);
    if (pointer || (alternate && base == 16 && length != 0)) {
        format_emit(state, '0'); format_emit(state, uppercase ? 'X' : 'x');
    } else if (alternate && base == 8 && (length == 0 || digits[length - 1] != '0')) {
        format_emit(state, '0');
    }
    format_repeat(state, '0', zeros);
    while (length) format_emit(state, digits[--length]);
    if (left) format_repeat(state, ' ', padding);
}

static int format_run(format_state_t *state, const char *format, va_list arguments) {
    while (*format) {
        if (*format != '%') { format_emit(state, *format++); continue; }
        format++;
        if (*format == '%') { format_emit(state, *format++); continue; }
        int left = 0, plus = 0, space = 0, alternate = 0, zero = 0;
        for (;;) {
            if (*format == '-') left = 1;
            else if (*format == '+') plus = 1;
            else if (*format == ' ') space = 1;
            else if (*format == '#') alternate = 1;
            else if (*format == '0') zero = 1;
            else break;
            format++;
        }
        int width = 0;
        if (*format == '*') { width = va_arg(arguments, int); format++; if (width < 0) { left = 1; width = -width; } }
        else while (*format >= '0' && *format <= '9') { width = width * 10 + (*format++ - '0'); }
        int precision = -1;
        if (*format == '.') {
            format++; precision = 0;
            if (*format == '*') { precision = va_arg(arguments, int); format++; if (precision < 0) precision = -1; }
            else while (*format >= '0' && *format <= '9') precision = precision * 10 + (*format++ - '0');
        }
        int long_count = 0;
        if (*format == 'h') { format++; if (*format == 'h') format++; }
        else if (*format == 'l') { long_count = 1; format++; if (*format == 'l') { long_count = 2; format++; } }
        else if (*format == 'z' || *format == 't' || *format == 'j') { long_count = 1; format++; }
        char conversion = *format ? *format++ : '\0';
        if (conversion == 's') {
            const char *text = va_arg(arguments, const char *);
            format_string(state, text ? text : "(null)", width, precision, left);
        } else if (conversion == 'c') {
            char value = (char)va_arg(arguments, int);
            int padding = width > 1 ? width - 1 : 0;
            if (!left) format_repeat(state, ' ', (size_t)padding);
            format_emit(state, value);
            if (left) format_repeat(state, ' ', (size_t)padding);
        } else if (conversion == 'd' || conversion == 'i') {
            long long value = long_count == 2 ? va_arg(arguments, long long) :
                              long_count == 1 ? va_arg(arguments, long) : va_arg(arguments, int);
            int negative = value < 0;
            unsigned long long magnitude = negative ? 0ull - (unsigned long long)value : (unsigned long long)value;
            format_number(state, magnitude, negative, 10, 0, width, precision, left, zero, alternate, plus, space, 0);
        } else if (conversion == 'u' || conversion == 'o' || conversion == 'x' || conversion == 'X' || conversion == 'p') {
            unsigned long long value = conversion == 'p' ? (unsigned int)va_arg(arguments, void *) :
                                      long_count == 2 ? va_arg(arguments, unsigned long long) :
                                      long_count == 1 ? va_arg(arguments, unsigned long) : va_arg(arguments, unsigned int);
            unsigned int base = conversion == 'o' ? 8 : (conversion == 'x' || conversion == 'X' || conversion == 'p') ? 16 : 10;
            format_number(state, value, 0, base, conversion == 'X', width, precision, left, zero, alternate, plus, space, conversion == 'p');
        } else {
            format_emit(state, '%');
            if (conversion) format_emit(state, conversion);
        }
    }
    if (state->buffer && state->capacity) {
        size_t end = state->count < state->capacity - 1 ? state->count : state->capacity - 1;
        state->buffer[end] = '\0';
    } else if (!state->buffer && state->chunk_length) {
        if (fwrite(state->chunk, 1, state->chunk_length, state->stream) != state->chunk_length)
            state->error = 1;
    }
    return state->error ? -1 : (int)state->count;
}

int vfprintf(FILE *stream, const char *format, va_list args) {
    if (!stream || !stream->writable) return -1;
    format_state_t state = {stream, (char *)0, 0, 0, 0, {0}, 0};
    va_list copy;
    va_copy(copy, args);
    int result = format_run(&state, format, copy);
    va_end(copy);
    return result;
}

int fprintf(FILE *stream, const char *format, ...) {
    va_list args; va_start(args, format);
    int result = vfprintf(stream, format, args);
    va_end(args); return result;
}

int printf(const char *format, ...) {
    va_list args; va_start(args, format);
    int result = vfprintf(stdout, format, args);
    va_end(args); return result;
}

int vsnprintf(char *buffer, size_t length, const char *format, va_list args) {
    char empty_buffer;
    format_state_t state = {(FILE *)0, buffer ? buffer : &empty_buffer, length, 0, 0, {0}, 0};
    va_list copy; va_copy(copy, args);
    int result = format_run(&state, format, copy);
    va_end(copy); return result;
}

int snprintf(char *buffer, size_t length, const char *format, ...) {
    va_list args; va_start(args, format);
    int result = vsnprintf(buffer, length, format, args);
    va_end(args); return result;
}

int vsprintf(char *buffer, const char *format, va_list args) {
    return vsnprintf(buffer, (size_t)-1, format, args);
}

int sprintf(char *buffer, const char *format, ...) {
    va_list args; va_start(args, format);
    int result = vsprintf(buffer, format, args);
    va_end(args); return result;
}
