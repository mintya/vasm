; M6 fixture：教材 §16 直接定址表。
; 把 3 个 handler 入口放在数据段，main 根据 al 索引调对应 handler。
data segment
    table dw h0, h1, h2     ; 函数指针表
data ends

stack segment
    db 64 dup (0)
stack ends

code segment
    assume cs:code, ds:data, ss:stack
start:
    mov ax, stack
    mov ss, ax
    mov sp, 64
    mov ax, data
    mov ds, ax

    ; 索引 = 1，调 h1
    mov bx, 1
    add bx, bx              ; *2（每个表项是 word）
    call word ptr ds:[bx + offset table]
    hlt

h0:
    mov ax, 100
    ret
h1:
    mov ax, 200
    ret
h2:
    mov ax, 300
    ret
code ends
end start
