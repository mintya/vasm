; M5 fixture：教材 §13 经典 hello world，用 DOS int 21h ah=09h 输出 '$' 结尾字符串。
data segment
    msg db 'Hello, world!$'
data ends

code segment
    assume cs:code, ds:data
start:
    mov ax, data
    mov ds, ax
    mov dx, offset msg
    mov ah, 9
    int 21h
    mov ah, 4ch
    int 21h
code ends
end start
