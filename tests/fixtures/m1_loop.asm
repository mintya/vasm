; M1 fixture: 教材 §5 loop
code segment
start:
    mov cx, 10
    mov ax, 0
loop_top:
    add ax, cx
    loop loop_top
code ends
end start
