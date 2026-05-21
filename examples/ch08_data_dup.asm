; 教材 §8 fixture：数据处理两个基本问题——字符串大小写转换。
; 教学点：db 'string' 字面量、loop 遍历、按位 or 把大写转小写（差 0x20）。
; 转完后第一个字节 al = 'h'（68h）。
data segment
    msg db 'HELLO'       ; 5 个大写字母
data ends

code segment
    assume cs:code, ds:data
start:
    mov ax, data
    mov ds, ax
    mov bx, 0            ; 当前下标
    mov cx, 5            ; 字符数
to_lower:
    mov al, [bx]
    or  al, 20h          ; 'A'(41h) | 20h = 'a'(61h)
    mov [bx], al
    inc bx
    loop to_lower
    mov al, [0]          ; 取首字符验证：应是 'h' = 68h
    hlt
code ends
end start
