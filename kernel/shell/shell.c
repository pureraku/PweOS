#include "shell/shell.h"
#include "drivers/vga.h"
#include "drivers/timer.h"
#include "drivers/keyboard.h"
#include "drivers/rtc.h"
#include "mm/heap.h"
#include "fs/ramfs.h"
#include "arch/cpu.h"
#include "arch/io.h"
#include "lib/printf.h"
#include "lib/string.h"

#define PWEOS_VERSION "0.1.0"

static void cmd_help(int argc, char **argv);
static void cmd_clear(int argc, char **argv);
static void cmd_echo(int argc, char **argv);
static void cmd_version(int argc, char **argv);
static void cmd_uptime(int argc, char **argv);
static void cmd_calc(int argc, char **argv);
static void cmd_color(int argc, char **argv);
static void cmd_sleep(int argc, char **argv);
static void cmd_peek(int argc, char **argv);
static void cmd_reboot(int argc, char **argv);
static void cmd_panic(int argc, char **argv);
static void cmd_about(int argc, char **argv);
static void cmd_meminfo(int argc, char **argv);
static void cmd_date(int argc, char **argv);
static void cmd_cpuinfo(int argc, char **argv);
static void cmd_ls(int argc, char **argv);
static void cmd_cat(int argc, char **argv);
static void cmd_touch(int argc, char **argv);
static void cmd_write(int argc, char **argv);
static void cmd_rm(int argc, char **argv);
static void cmd_history(int argc, char **argv);

struct shell_command {
    const char *name;
    const char *desc;
    void (*handler)(int argc, char **argv);
};

static const struct shell_command commands[] = {
    {"help",    "Display list of available commands",      cmd_help},
    {"clear",   "Clear screen and reset cursor",          cmd_clear},
    {"echo",    "Print arguments to the console",         cmd_echo},
    {"version", "Display PweOS kernel version and arch",  cmd_version},
    {"uname",   "Display operating system information",   cmd_version},
    {"uptime",  "Display system uptime and timer ticks",  cmd_uptime},
    {"date",    "Display real-time clock (CMOS RTC) date",cmd_date},
    {"cpuinfo", "Display CPU hardware details via CPUID",  cmd_cpuinfo},
    {"meminfo", "Display kernel heap memory statistics",   cmd_meminfo},
    {"history", "Display command history buffer",         cmd_history},
    {"ls",      "List files in in-memory RAM filesystem", cmd_ls},
    {"cat",     "Display file contents: cat <filename>",  cmd_cat},
    {"touch",   "Create empty file: touch <filename>",    cmd_touch},
    {"write",   "Write to file: write <filename> <text>", cmd_write},
    {"rm",      "Delete file: rm <filename>",             cmd_rm},
    {"calc",    "Evaluate arithmetic expression: a op b", cmd_calc},
    {"color",   "Set terminal color: color <fg> [bg]",    cmd_color},
    {"sleep",   "Delay execution for N seconds",          cmd_sleep},
    {"peek",    "Read byte from memory: peek <hex_addr>", cmd_peek},
    {"reboot",  "Reboot the system",                      cmd_reboot},
    {"panic",   "Trigger kernel panic (exception test)",  cmd_panic},
    {"about",   "Information about PweOS system",         cmd_about},
};

static const size_t command_count = sizeof(commands) / sizeof(commands[0]);

static void cmd_help(int argc, char **argv) {
    (void)argc;
    (void)argv;
    printf("\nPweOS Builtin Commands:\n");
    printf("-----------------------:3---------------------------\n");
    for (size_t i = 0; i < command_count; i++) {
        vga_set_color(VGA_COLOR_LIGHT_GREEN, VGA_COLOR_BLACK);
        printf("  %-10s", commands[i].name);
        vga_set_color(VGA_COLOR_LIGHT_GREY, VGA_COLOR_BLACK);
        printf(" - %s\n", commands[i].desc);
    }
    printf("-----------------------:3----------------------------\n");
}

static void cmd_clear(int argc, char **argv) {
    (void)argc;
    (void)argv;
    vga_clear();
}

static void cmd_echo(int argc, char **argv) {
    for (int i = 1; i < argc; i++) {
        printf("%s%s", argv[i], (i == argc - 1) ? "" : " ");
    }
    printf("\n");
}

static void cmd_version(int argc, char **argv) {
    (void)argc;
    (void)argv;
    printf("PweOS :3 version %s (i686-pc-none-elf)\n", PWEOS_VERSION);
    printf("Build: Freestanding 32-bit x86 Kernel\n");
}

static void cmd_uptime(int argc, char **argv) {
    (void)argc;
    (void)argv;
    uint32_t ticks = timer_get_ticks();
    uint32_t secs = timer_get_uptime_seconds();
    uint32_t mins = secs / 60;
    uint32_t rem_secs = secs % 60;

    printf("Uptime: %u min, %u sec (%u timer ticks elapsed at 100 Hz) :3\n", mins, rem_secs, ticks);
}

static void cmd_date(int argc, char **argv) {
    (void)argc;
    (void)argv;
    rtc_datetime_t dt;
    rtc_get_datetime(&dt);

    printf("%04u-%02u-%02u %02u:%02u:%02u UTC :3\n",
           (unsigned int)dt.year,
           (unsigned int)dt.month,
           (unsigned int)dt.day,
           (unsigned int)dt.hour,
           (unsigned int)dt.minute,
           (unsigned int)dt.second);
}

static void cmd_cpuinfo(int argc, char **argv) {
    (void)argc;
    (void)argv;
    cpu_info_t info;
    cpu_get_info(&info);

    printf("\nCPU Hardware Information:\n");
    printf("----------------------------------------------------\n");
    printf("  Vendor ID        : %s\n", info.vendor);
    printf("  Model / Brand    : %s\n", info.brand);
    printf("  Family / Model   : Family %u, Model %u, Stepping %u\n",
           (unsigned int)info.family, (unsigned int)info.model, (unsigned int)info.stepping);
    printf("  Features         : %s%s%s%s%s%s\n",
           info.has_fpu ? "FPU " : "",
           info.has_tsc ? "TSC " : "",
           info.has_msr ? "MSR " : "",
           info.has_apic ? "APIC " : "",
           info.has_sse ? "SSE " : "",
           info.has_sse2 ? "SSE2 " : "");
    printf("  Hypervisor State : %s\n", info.has_hypervisor ? "Virtual Machine Detected" : "Bare Metal");
    printf("----------------------------------------------------\n");
}

static void print_file_entry(const char *name, size_t size) {
    vga_set_color(VGA_COLOR_LIGHT_CYAN, VGA_COLOR_BLACK);
    printf("  %-16s", name);
    vga_set_color(VGA_COLOR_LIGHT_GREY, VGA_COLOR_BLACK);
    printf(" %u bytes\n", (unsigned int)size);
}

static void cmd_ls(int argc, char **argv) {
    (void)argc;
    (void)argv;
    printf("\nRAMFS Files (%u total):\n", (unsigned int)ramfs_count());
    printf("dang so many files..\n");
    ramfs_list(print_file_entry);
    printf("----------------------------------------------------\n");
}

static void cmd_cat(int argc, char **argv) {
    if (argc < 2) {
        printf("Usage: cat <filename>\n");
        return;
    }
    size_t size = 0;
    const char *data = ramfs_read(argv[1], &size);
    if (!data) {
        vga_set_color(VGA_COLOR_LIGHT_RED, VGA_COLOR_BLACK);
        printf("Error: File '%s' not found.\n", argv[1]);
        vga_set_color(VGA_COLOR_LIGHT_GREY, VGA_COLOR_BLACK);
        return;
    }
    printf("%s", data);
    if (size > 0 && data[size - 1] != '\n') {
        printf("\n");
    }
}

static void cmd_touch(int argc, char **argv) {
    if (argc < 2) {
        printf("Usage: touch <filename>\n");
        return;
    }
    if (ramfs_create(argv[1], "")) {
        printf("Created file '%s'. :3\n", argv[1]);
    } else {
        printf("File '%s' already exists or creation failed.\n", argv[1]);
    }
}

static void cmd_write(int argc, char **argv) {
    if (argc < 3) {
        printf("Usage: write <filename> <text...>\n");
        return;
    }

    // Join remaining arguments into single string
    char buffer[256];
    buffer[0] = '\0';
    size_t pos = 0;
    for (int i = 2; i < argc; i++) {
        size_t arg_len = strlen(argv[i]);
        if (pos + arg_len + 2 >= sizeof(buffer)) break;
        if (i > 2) buffer[pos++] = ' ';
        strcpy(buffer + pos, argv[i]);
        pos += arg_len;
    }
    buffer[pos++] = '\n';
    buffer[pos] = '\0';

    if (ramfs_write(argv[1], buffer, false)) {
        printf("Wrote %u bytes to '%s'. :3\n", (unsigned int)pos, argv[1]);
    } else {
        printf("Failed to write to '%s'. :(\n", argv[1]);
    }
}

static void cmd_rm(int argc, char **argv) {
    if (argc < 2) {
        printf("Usage: rm <filename>\n");
        return;
    }
    if (ramfs_delete(argv[1])) {
        printf("Deleted file '%s'. :3\n", argv[1]);
    } else {
        vga_set_color(VGA_COLOR_LIGHT_RED, VGA_COLOR_BLACK);
        printf("Error: File '%s' not found.\n", argv[1]);
        vga_set_color(VGA_COLOR_LIGHT_GREY, VGA_COLOR_BLACK);
    }
}

static void cmd_meminfo(int argc, char **argv) {
    (void)argc;
    (void)argv;
    heap_stats_t stats;
    heap_get_stats(&stats);

    printf("\nKernel Heap Memory Statistics:\n");
    printf("----------------------------------------------------\n");
    printf("  Total Heap Capacity : %u KB (%u bytes)\n", (unsigned int)(stats.total_bytes / 1024), (unsigned int)stats.total_bytes);
    printf("  Used Memory         : %u bytes\n", (unsigned int)stats.used_bytes);
    printf("  Free Memory         : %u bytes\n", (unsigned int)stats.free_bytes);
    printf("  Active Allocations  : %u\n", (unsigned int)stats.num_allocations);
    printf("  Total Heap Blocks   : %u\n", (unsigned int)stats.num_blocks);
    printf("----------------------------------------------------\n");
}

static void cmd_calc(int argc, char **argv) {
    if (argc < 4) {
        printf("Usage: calc <number> <+|-|*|/|%%> <number>\n");
        printf("Example: calc 15 * 7\n");
        return;
    }

    int a = atoi(argv[1]);
    char op = argv[2][0];
    int b = atoi(argv[3]);
    int result = 0;

    switch (op) {
        case '+':
            result = a + b;
            printf("%d + %d = %d\n", a, b, result);
            break;
        case '-':
            result = a - b;
            printf("%d - %d = %d\n", a, b, result);
            break;
        case '*':
            result = a * b;
            printf("%d * %d = %d\n", a, b, result);
            break;
        case '/':
            if (b == 0) {
                vga_set_color(VGA_COLOR_LIGHT_RED, VGA_COLOR_BLACK);
                printf("Error: Division by zero!\n");
                vga_set_color(VGA_COLOR_LIGHT_GREY, VGA_COLOR_BLACK);
                return;
            }
            result = a / b;
            printf("%d / %d = %d (remainder %d)\n", a, b, result, a % b);
            break;
        case '%':
            if (b == 0) {
                vga_set_color(VGA_COLOR_LIGHT_RED, VGA_COLOR_BLACK);
                printf("Error: Division by zero!\n");
                vga_set_color(VGA_COLOR_LIGHT_GREY, VGA_COLOR_BLACK);
                return;
            }
            result = a % b;
            printf("%d %% %d = %d\n", a, b, result);
            break;
        default:
            printf("Unknown operator '%c'. Supported: +, -, *, /, %%\n", op);
            break;
    }
}

static int parse_color_name(const char *name) {
    if (strcmp(name, "black") == 0)        return VGA_COLOR_BLACK;
    if (strcmp(name, "blue") == 0)         return VGA_COLOR_BLUE;
    if (strcmp(name, "green") == 0)        return VGA_COLOR_GREEN;
    if (strcmp(name, "cyan") == 0)         return VGA_COLOR_CYAN;
    if (strcmp(name, "red") == 0)          return VGA_COLOR_RED;
    if (strcmp(name, "magenta") == 0)      return VGA_COLOR_MAGENTA;
    if (strcmp(name, "brown") == 0)        return VGA_COLOR_BROWN;
    if (strcmp(name, "grey") == 0)         return VGA_COLOR_LIGHT_GREY;
    if (strcmp(name, "darkgrey") == 0)     return VGA_COLOR_DARK_GREY;
    if (strcmp(name, "lightblue") == 0)    return VGA_COLOR_LIGHT_BLUE;
    if (strcmp(name, "lightgreen") == 0)   return VGA_COLOR_LIGHT_GREEN;
    if (strcmp(name, "lightcyan") == 0)    return VGA_COLOR_LIGHT_CYAN;
    if (strcmp(name, "lightred") == 0)     return VGA_COLOR_LIGHT_RED;
    if (strcmp(name, "lightmagenta") == 0) return VGA_COLOR_LIGHT_MAGENTA;
    if (strcmp(name, "yellow") == 0)       return VGA_COLOR_LIGHT_BROWN;
    if (strcmp(name, "white") == 0)        return VGA_COLOR_WHITE;
    return -1;
}

static void cmd_color(int argc, char **argv) {
    if (argc < 2) {
        printf("Usage: color <fg> [bg]\n");
        printf("Colors: black, blue, green, cyan, red, magenta, brown, grey,\n");
        printf("        darkgrey, lightblue, lightgreen, lightcyan, lightred,\n");
        printf("        lightmagenta, yellow, white\n");
        return;
    }

    int fg = parse_color_name(argv[1]);
    if (fg < 0) {
        printf("Unknown foreground color '%s'\n", argv[1]);
        return;
    }

    int bg = VGA_COLOR_BLACK;
    if (argc >= 3) {
        bg = parse_color_name(argv[2]);
        if (bg < 0) {
            printf("Unknown background color '%s'\n", argv[2]);
            return;
        }
    }

    vga_set_color((enum vga_color)fg, (enum vga_color)bg);
    printf("Color changed to %s on %s. :3\n", argv[1], argc >= 3 ? argv[2] : "black");
}

static void cmd_sleep(int argc, char **argv) {
    if (argc < 2) {
        printf("Usage: sleep <seconds>\n");
        return;
    }
    int s = atoi(argv[1]);
    if (s <= 0) s = 1;
    printf("Sleeping for %d second(s)... :3\n", s);
    timer_sleep((uint32_t)(s * 100));
    printf("Done. :3\n");
}

static uint32_t parse_hex(const char *s) {
    if (s[0] == '0' && (s[1] == 'x' || s[1] == 'X')) s += 2;
    uint32_t val = 0;
    while (*s) {
        val <<= 4;
        if (*s >= '0' && *s <= '9') val |= (*s - '0');
        else if (*s >= 'a' && *s <= 'f') val |= (*s - 'a' + 10);
        else if (*s >= 'A' && *s <= 'F') val |= (*s - 'A' + 10);
        else break;
        s++;
    }
    return val;
}

static void cmd_peek(int argc, char **argv) {
    if (argc < 2) {
        printf("Usage: peek <hex_address>\n");
        printf("Example: peek 0xb8000\n");
        return;
    }
    uint32_t addr = parse_hex(argv[1]);
    volatile uint8_t *ptr = (volatile uint8_t *)addr;
    printf("Memory [0x%08x] = 0x%02x ('%c')\n", addr, *ptr, (*ptr >= 32 && *ptr <= 126) ? *ptr : '.');
}

static void cmd_reboot(int argc, char **argv) {
    (void)argc;
    (void)argv;
    printf("Restarting PweOS :3...\n");
    timer_sleep(20);

    // Pulse CPU reset line via 8042 keyboard controller
    uint8_t temp;
    do {
        temp = inb(0x64);
        if (temp & 1) inb(0x60);
    } while (temp & 2);
    outb(0x64, 0xFE);

    // If hardware reset didn't trigger, cause triple fault via zero IDT
    struct { uint16_t limit; uint32_t base; } __attribute__((packed)) null_idt = {0, 0};
    __asm__ volatile ("cli; lidt %0; int $3" : : "m"(null_idt));
}

static void cmd_panic(int argc, char **argv) {
    (void)argc;
    (void)argv;
    printf("Triggering division by zero exception...\n");
    volatile int zero = 0;
    volatile int x = 42 / zero;
    (void)x;
}

static void cmd_about(int argc, char **argv) {
    (void)argc;
    (void)argv;
    vga_set_color(VGA_COLOR_LIGHT_CYAN, VGA_COLOR_BLACK);
    printf("\n  ==== About PweOS :3 ====\n");
    vga_set_color(VGA_COLOR_WHITE, VGA_COLOR_BLACK);
    printf("  Architecture : 32-bit x86 (i686 Protected Mode)\n");
    printf("  Bootloader   : 2-stage custom BIOS MBR with A20 & GDT\n");
    printf("  Kernel       : Freestanding C runtime with custom libc\n");
    printf("  Drivers      : VGA Text Mode, PS/2 Keyboard, PIT 8253, Serial COM1\n");
    printf("  Interrupts   : 8259 PIC Remapped + 256 IDT Descriptors\n");
    printf("  Shell        : Interactive command interpreter with line editing\n\n");
    vga_set_color(VGA_COLOR_LIGHT_GREY, VGA_COLOR_BLACK);
}

void shell_init(void) {
    vga_set_color(VGA_COLOR_YELLOW, VGA_COLOR_BLACK);
    printf("Type 'help' to see available commands.\n\n");
    vga_set_color(VGA_COLOR_LIGHT_GREY, VGA_COLOR_BLACK);
}

#define SHELL_HISTORY_MAX 16
static char shell_history[SHELL_HISTORY_MAX][SHELL_MAX_LINE];
static size_t shell_history_count = 0;

static void history_add(const char *cmd) {
    if (!cmd || strlen(cmd) == 0) return;
    if (shell_history_count > 0 && strcmp(shell_history[shell_history_count - 1], cmd) == 0) {
        return; // Avoid adjacent duplicate entries
    }
    if (shell_history_count < SHELL_HISTORY_MAX) {
        strcpy(shell_history[shell_history_count++], cmd);
    } else {
        for (size_t i = 0; i < SHELL_HISTORY_MAX - 1; i++) {
            strcpy(shell_history[i], shell_history[i + 1]);
        }
        strcpy(shell_history[SHELL_HISTORY_MAX - 1], cmd);
    }
}

static void cmd_history(int argc, char **argv) {
    (void)argc;
    (void)argv;
    printf("\nCommand History: :3\n");
    printf("----------------------------------------------------\n");
    if (shell_history_count == 0) {
        printf("  (no commands in history)\n");
    } else {
        for (size_t i = 0; i < shell_history_count; i++) {
            printf("  %2u  %s\n", (unsigned int)(i + 1), shell_history[i]);
        }
    }
    printf("----------------------------------------------------\n");
}

static size_t shell_readline(char *buffer, size_t max_len) {
    if (!buffer || max_len == 0) return 0;
    size_t index = 0;
    int history_pos = (int)shell_history_count;

    for (;;) {
        char c = keyboard_getchar();

        if (c == '\n' || c == '\r') {
            vga_putchar('\n');
            buffer[index] = '\0';
            return index;
        } else if (c == '\b') {
            if (index > 0) {
                index--;
                vga_putchar('\b');
            }
        } else if (c == KEY_UP) {
            if (history_pos > 0) {
                history_pos--;
                while (index > 0) {
                    vga_putchar('\b');
                    index--;
                }
                const char *hist = shell_history[history_pos];
                size_t len = strlen(hist);
                for (size_t i = 0; i < len && i < max_len - 1; i++) {
                    buffer[i] = hist[i];
                    vga_putchar(hist[i]);
                }
                index = len;
            }
        } else if (c == KEY_DOWN) {
            if (history_pos + 1 < (int)shell_history_count) {
                history_pos++;
                while (index > 0) {
                    vga_putchar('\b');
                    index--;
                }
                const char *hist = shell_history[history_pos];
                size_t len = strlen(hist);
                for (size_t i = 0; i < len && i < max_len - 1; i++) {
                    buffer[i] = hist[i];
                    vga_putchar(hist[i]);
                }
                index = len;
            } else if (history_pos < (int)shell_history_count) {
                history_pos = (int)shell_history_count;
                while (index > 0) {
                    vga_putchar('\b');
                    index--;
                }
            }
        } else if ((unsigned char)c >= 32 && (unsigned char)c <= 126) {
            if (index + 1 < max_len) {
                buffer[index++] = c;
                vga_putchar(c);
            }
        }
    }
}

void shell_run(void) {
    char line_buffer[SHELL_MAX_LINE];
    char saved_line[SHELL_MAX_LINE];
    char *argv[SHELL_MAX_ARGS];

    for (;;) {
        // Render colored shell prompt
        vga_set_color(VGA_COLOR_LIGHT_CYAN, VGA_COLOR_BLACK);
        printf("pweos");
        vga_set_color(VGA_COLOR_LIGHT_GREEN, VGA_COLOR_BLACK);
        printf("> ");
        vga_set_color(VGA_COLOR_WHITE, VGA_COLOR_BLACK);

        // Read user input line with history support
        size_t len = shell_readline(line_buffer, sizeof(line_buffer));
        if (len == 0) {
            continue;
        }

        // Save raw command to history
        strcpy(saved_line, line_buffer);
        history_add(saved_line);

        // Parse line into argc, argv
        int argc = 0;
        char *p = line_buffer;
        while (*p && argc < SHELL_MAX_ARGS) {
            while (*p == ' ' || *p == '\t') {
                *p++ = '\0';
            }
            if (*p == '\0') break;
            argv[argc++] = p;
            while (*p && *p != ' ' && *p != '\t') {
                p++;
            }
        }

        if (argc == 0) {
            continue;
        }

        // Dispatch command
        bool found = false;
        for (size_t i = 0; i < command_count; i++) {
            if (strcmp(argv[0], commands[i].name) == 0) {
                commands[i].handler(argc, argv);
                found = true;
                break;
            }
        }

        if (!found) {
            vga_set_color(VGA_COLOR_LIGHT_RED, VGA_COLOR_BLACK);
            printf("Unknown command: '%s'. Type 'help' for command list.\n", argv[0]);
            vga_set_color(VGA_COLOR_LIGHT_GREY, VGA_COLOR_BLACK);
        }
    }
}
