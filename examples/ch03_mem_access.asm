; 教材 §3 fixture：通过 [bx] 访问内存。
; 教学点：把段值装入 ds、用 [bx] 读字节、字节寄存器 al/ah/bl。
; 终态：al = 41h ('A'), ah = 42h ('B')。
data segment
    letters db 'A', 'B', 'C', 'D'
data ends

code segment
    assume cs:code, ds:data
start:
    mov ax, data
    mov ds, ax
    mov bx, 0
    mov al, [bx]         ; al = letters[0] = 'A' = 41h
    inc bx
    mov ah, [bx]         ; ah = letters[1] = 'B' = 42h
    hlt
code ends
end start
