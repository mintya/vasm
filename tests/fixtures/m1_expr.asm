; M1 fixture: 表达式
data segment
    msg db 'hi$'
    nums dw 1 + 2
    expr dw (3 + 4) * 5
    sub_exp dw 100 - 50 / 2
    mod_exp dw 17 % 5
    sym_off dw offset msg
    sym_off_plus dw offset msg + 3
    seg_of dw seg msg
    neg_imm dw -7
data ends

code segment
    mov ax, offset msg
    mov bx, offset msg + 2
    mov cx, (1 + 2) * 3
    mov dx, 100h
code ends
end
