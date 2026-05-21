; M5 fixture：BIOS int 10h ah=09h 重复输出一个字符 cx 次。
code segment
    assume cs:code
start:
    mov al, '*'
    mov cx, 20
    mov ah, 9
    int 10h
    mov ah, 4ch
    int 21h
code ends
end start
