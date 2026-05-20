; M2 fixture: 多段程序 + 段寄存器赋值 + 数据段读取
; 加载 data 段值到 ds，读 nums 数组前三个 word，相加到 ax
data segment
    nums dw 1, 2, 3, 4
data ends

stack segment
    db 64 dup (0)
stack ends

code segment
    assume cs:code, ds:data, ss:stack
start:
    mov ax, data         ; 段值
    mov ds, ax
    mov ax, stack
    mov ss, ax
    mov sp, 64

    mov si, 0
    mov ax, [si]         ; ds:[si] = nums[0] = 1
    add si, 2
    add ax, [si]         ; + nums[1] = 2 → ax = 3
    add si, 2
    add ax, [si]         ; + nums[2] = 3 → ax = 6

    push ax              ; 验证栈：sp -= 2，[ss:sp] = 6
    pop bx               ; bx = 6，sp += 2
    hlt
code ends
end start
