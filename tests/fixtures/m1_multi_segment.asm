; M1 fixture: 教材 §6 多段程序
data segment
    msg db 'hello, world$'
data ends

stack segment
    db 64 dup (0)
stack ends

code segment
    assume cs:code, ds:data, ss:stack
start:
    mov ax, data
    mov ds, ax
    mov ax, stack
    mov ss, ax
    mov sp, 64
    mov ah, 9
    mov dx, offset msg
    int 21h
    mov ax, 4c00h
    int 21h
code ends
end start
