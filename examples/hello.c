typedef unsigned int u32;

static volatile u32 exit_status = 0;

static void write_text(const char *text, u32 length) {
    u32 result = 1;
    __asm__ volatile (
        "int $0x80"
        : "+a"(result)
        : "b"(text), "c"(length)
        : "memory"
    );
}

static void exit_process(u32 status) __attribute__((noreturn));
static void exit_process(u32 status) {
    u32 call = 0;
    __asm__ volatile (
        "int $0x80"
        : "+a"(call)
        : "b"(status)
        : "memory"
    );
    for (;;) {
        __asm__ volatile ("hlt");
    }
}

void _start(void) {
    static const char message[] = "Hello from C in PweOS user mode!\n";
    write_text(message, sizeof(message) - 1);
    exit_process(exit_status);
}
