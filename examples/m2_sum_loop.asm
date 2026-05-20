; M2 fixture: 求和 1+2+...+10 = 55，用 loop + add
; 验证 add / sub / inc / dec / loop / hlt 与 ip 跳转
code segment
start:
    mov ax, 0          ; 累加器
    mov cx, 10         ; 循环次数
    mov bx, 1          ; 当前加数
sum_loop:
    add ax, bx
    inc bx
    loop sum_loop
    hlt
code ends
end start
