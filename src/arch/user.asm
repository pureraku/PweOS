; GDT replacement and ring-3 entry helpers.
[bits 32]

global gdt_flush
global user_enter
global user_leave
extern USER_RETURN_ESP
extern USER_EXIT_STATUS
extern USER_RETURN_EFLAGS
extern gdt_set_kernel_stack

gdt_flush:
    mov eax, [esp + 4]
    lgdt [eax]
    jmp 0x08:.reload_segments
.reload_segments:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    mov ax, 0x28
    ltr ax
    ret

; user_enter(entry, stack) returns after user_leave(status).
user_enter:
    push ebp
    push ebx
    push esi
    push edi
    pushfd
    pop eax
    mov [USER_RETURN_EFLAGS], eax
    mov eax, esp
    push eax
    call gdt_set_kernel_stack
    add esp, 4
    mov [USER_RETURN_ESP], esp

    mov esi, [esp + 20]       ; entry (arguments are above saved registers)
    mov edx, [esp + 24]       ; user stack
    mov ax, 0x23
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    push dword 0x23
    push edx
    pushfd
    pop ecx
    or ecx, 0x200
    push ecx
    push dword 0x1b
    push esi
    iret

; Called from the syscall/exception handler to resume the shell's kernel stack.
user_leave:
    mov eax, [esp + 4]
    mov [USER_EXIT_STATUS], eax
    mov esp, [USER_RETURN_ESP]
    mov eax, [USER_EXIT_STATUS]
    pop edi
    pop esi
    pop ebx
    pop ebp
    push dword [USER_RETURN_EFLAGS]
    popfd
    ret
