/*
 * PweOS TinyCC capability tour.
 * Build: tcc /demo.c -o demo
 * Run:   demo [arguments...]
 * Creates /demo-data.txt briefly, then removes it.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <inttypes.h>
#include <stdbool.h>
#include <stdarg.h>
#include <stdalign.h>
#include <stdatomic.h>
#include <stddef.h>
#include <ctype.h>
#include <errno.h>
#include <fcntl.h>
#include <unistd.h>
#include <time.h>
#include <sys/time.h>
#include <math.h>
#include <setjmp.h>

#define ARRAY_COUNT(a) (sizeof(a) / sizeof((a)[0]))
#define TRACE(fmt, ...) printf("[trace] " fmt "\n", ##__VA_ARGS__)
#define STRINGIFY_INNER(value) #value
#define STRINGIFY(value) STRINGIFY_INNER(value)

/* GNU attributes, typedefs, enums, unions, bitfields and designated init. */
typedef struct __attribute__((packed)) packet_header {
    uint8_t kind;
    uint16_t length;
    unsigned int urgent : 1;
    unsigned int reserved : 7;
} packet_header_t;

typedef struct aligned_value {
    uint32_t value;
} __attribute__((aligned(16))) aligned_value_t;

typedef union number_view {
    uint32_t word;
    unsigned char bytes[4];
} number_view_t;

enum demo_mode { MODE_IDLE = 1, MODE_RUNNING = 2, MODE_DONE = 4 };

static int compare_ints(const void *left, const void *right) {
    int a = *(const int *)left, b = *(const int *)right;
    return (a > b) - (a < b);
}

static inline int square(int value) { return value * value; }
static int likely_positive(int value) {
    if (__builtin_expect(value > 0, 1)) return value;
    return -value;
}

/* Attribute and variadic argument handling. */
static void log_message(const char *format, ...) __attribute__((format(printf, 1, 2)));
static void log_message(const char *format, ...) {
    va_list args;
    va_start(args, format);
    fputs("log: ", stdout);
    vfprintf(stdout, format, args);
    putchar('\n');
    va_end(args);
}

static void show_language_and_compiler(void) {
    int values[] = {9, 2, 7, 1, 5};
    uint32_t bits = UINT32_C(0x2a);
    char scratch[32];
    char *end = NULL;
    long parsed = strtol("12345tail", &end, 10);
    double fraction = strtod("12.5", NULL);
    volatile int asm_result = 0;
    packet_header_t header = {.kind = 3, .length = 12, .urgent = 1};
    number_view_t view = {.word = 0x12345678};
    aligned_value_t aligned = { .value = 99 };

    puts("\n== compiler, C language and libc ==");
    printf("TinyCC version macro=%s; C standard=%ld; target=%s\n",
           STRINGIFY(__TINYC__), (long)__STDC_VERSION__,
#if defined(__i386__)
           "i386"
#else
           "other"
#endif
    );
    printf("sizeof(void*)=%u sizeof(long)=%u, bool=%d\n",
           (unsigned)sizeof(void *), (unsigned)sizeof(long), true);
    printf("packed header size=%u offset(length)=%u urgent=%u; aligned=%u\n",
           (unsigned)sizeof(header), (unsigned)offsetof(packet_header_t, length),
           header.urgent, (unsigned)__alignof__(aligned));
    printf("union bytes=%02x %02x %02x %02x; enum=%d\n",
           view.bytes[0], view.bytes[1], view.bytes[2], view.bytes[3], MODE_RUNNING);
    printf("integer formats: dec=%" PRId64 " hex=%#x u64=%" PRIu64 "\n",
           (int64_t)12345678901LL, bits, (uint64_t)12345678901ULL);
    printf("strtol=%ld suffix='%s'; strtod scaled x10=%d\n",
           parsed, end, (int)(fraction * 10.0));
    printf("ctype: lower(P)=%c digit(7)=%d; square(6)=%d likely=%d\n",
           tolower('P'), isdigit('7'), square(6), likely_positive(8));
    snprintf(scratch, sizeof(scratch), "%08x:%-6s", bits, "ready");
    printf("snprintf: %s (%u chars)\n", scratch, (unsigned)strlen(scratch));
    qsort(values, ARRAY_COUNT(values), sizeof(values[0]), compare_ints);
    printf("qsort:");
    for (size_t i = 0; i < ARRAY_COUNT(values); i++) printf(" %d", values[i]);
    printf("; builtin popcount=%d ctz=%d ffs=%d\n",
           __builtin_popcount(bits), __builtin_ctz(bits), __builtin_ffs((int)bits));
    printf("builtin clz=%d parity=%d; float math ldexp(1.5,3)*10=%d\n",
           __builtin_clz(bits), __builtin_parity(bits), (int)(ldexp(1.5, 3) * 10));

    /* GNU statement expression, typeof and inline i386 assembly. */
    int statement_expr = ({ typeof(bits) temporary = bits; (int)(temporary + 1); });
    __asm__ volatile ("movl $7, %0" : "=r" (asm_result) : : "memory");
    printf("GNU extensions: statement-expression=%d typeof-size=%u asm=%d\n",
           statement_expr, (unsigned)sizeof(typeof(bits)), asm_result);
    TRACE("variadic macro says %s", "hello");
}

static void show_memory_and_control_flow(void) {
    char *memory = (char *)malloc(12);
    unsigned char *zeroed;
    jmp_buf environment;
    int jump_value;
    atomic_int counter;
    int old_value, new_value;

    puts("\n== heap, setjmp and atomics ==");
    if (!memory) { perror("malloc"); return; }
    strcpy(memory, "allocated");
    memory = (char *)realloc(memory, 32);
    if (memory) {
        strcat(memory, " memory");
        zeroed = (unsigned char *)calloc(1, 1);
        printf("malloc/realloc: %s; calloc zero=%d\n", memory,
               zeroed ? zeroed[0] : -1);
        free(zeroed);
    }
    free(memory);

    jump_value = setjmp(environment);
    if (jump_value == 0) longjmp(environment, 7);
    printf("setjmp/longjmp returned %d\n", jump_value);

    atomic_init(&counter, 10);
    old_value = atomic_fetch_add(&counter, 5);
    new_value = atomic_load(&counter);
    printf("atomic fetch_add old=%d new=%d\n", old_value, new_value);
}

static void show_stdio_file(void) {
    const char *path = "/demo-data.txt";
    FILE *file = fopen(path, "w+");
    char line[64];
    if (!file) { perror("fopen"); return; }
    fprintf(file, "PweOS stdio demo\nnumber=%d\n", 42);
    printf("\n== stdio and RAMFS ==\nftell after write=%ld\n", ftell(file));
    rewind(file);
    while (fgets(line, sizeof(line), file)) printf("read: %s", line);
    printf("EOF=%d error=%d\n", feof(file), ferror(file));
    clearerr(file);
    fclose(file);
    if (remove(path) == 0) puts("stdio remove succeeded");
    else perror("remove");
}

static void show_syscalls(void) {
    char cwd[128], buffer[80];
    int fd;
    puts("\n== PweOS syscall wrappers ==");
    printf("cwd=%s stdin-isatty=%d\n",
           getcwd(cwd, sizeof(cwd)) ? cwd : "?", isatty(STDIN_FILENO));
    fd = open("/demo-data.txt", O_CREAT | O_TRUNC | O_RDWR, 0644);
    if (fd < 0) { perror("open"); return; }
    if (write(fd, "raw syscall I/O\n", 16) < 0) perror("write");
    if (lseek(fd, 0, SEEK_SET) >= 0) {
        ssize_t count = read(fd, buffer, sizeof(buffer) - 1);
        if (count >= 0) { buffer[count] = '\0'; printf("read: %s", buffer); }
        else perror("read");
    }
    close(fd);
    if (unlink("/demo-data.txt") < 0) perror("unlink");
}

static void show_clock(void) {
    struct timeval now;
    time_t seconds;
    struct tm *utc;
    puts("\n== clock ==");
    if (gettimeofday(&now, NULL) < 0) { perror("gettimeofday"); return; }
    seconds = time(NULL);
    utc = gmtime(&seconds);
    if (utc) printf("UTC %04d-%02d-%02d %02d:%02d:%02d (%ld seconds)\n",
                    utc->tm_year + 1900, utc->tm_mon + 1, utc->tm_mday,
                    utc->tm_hour, utc->tm_min, utc->tm_sec, now.tv_sec);
}

int main(int argc, char **argv) {
    puts("PweOS TinyCC demo");
    printf("argc=%d argv:", argc);
    for (int i = 0; i < argc; i++) printf(" [%s]", argv[i]);
    putchar('\n');
    log_message("user-mode C program started (%d argument%s)", argc, argc == 1 ? "" : "s");
    show_language_and_compiler();
    show_memory_and_control_flow();
    show_stdio_file();
    show_syscalls();
    show_clock();
    puts("\nDemo complete.");
    return EXIT_SUCCESS;
}
