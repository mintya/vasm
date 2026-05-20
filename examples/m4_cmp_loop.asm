; M4 fixture: 用 cmp + jne 实现倒数累加 1+2+...+10 = 55
; 验证条件跳转 / cmp 标志位 / inc/dec 联动
code segment
start:
    mov ax, 0          ; 累加器
    mov bx, 10         ; 当前加数
lp:
    add ax, bx
    dec bx
    cmp bx, 0
    jne lp
    hlt
code ends
end start
