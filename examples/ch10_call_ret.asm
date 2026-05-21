; M4 fixture: 子过程调用，验证 call / ret + 栈对称
; sub 把 ax += bx，主程序调用两次后 ax = bx*2
stack segment
    db 64 dup (0)
stack ends

code segment
    assume cs:code, ss:stack
start:
    mov ax, stack
    mov ss, ax
    mov sp, 64

    mov ax, 0
    mov bx, 100
    call add_bx_to_ax
    call add_bx_to_ax
    hlt

add_bx_to_ax:
    add ax, bx
    ret
code ends
end start
