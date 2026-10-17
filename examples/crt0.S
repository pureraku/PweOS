.section .text.entry,"ax",@progbits
.global _start
.type _start,@function
.extern main

_start:
    xorl %ebp, %ebp
    movl (%esp), %eax
    leal 4(%esp), %edx
    pushl %edx
    pushl %eax
    call main
    addl $8, %esp

    movl %eax, %ebx
    xorl %eax, %eax
    int $0x80
1:
    cli
    hlt
    jmp 1b
.size _start, .-_start
