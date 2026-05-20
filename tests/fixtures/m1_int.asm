; M1 fixture: 教材 §13 int 21h 调用
data segment
    msg db 'press any key$'
data ends
code segment
    assume cs:code, ds:data
start:
    mov ax, data
    mov ds, ax
    mov ah, 9
    mov dx, offset msg
    int 21h
    mov ah, 1
    int 21h
    mov ah, 4ch
    int 21h
code ends
end start
