; 教材 §4 fixture：第一个完整程序。
; 教学点：assume / segment-ends / end <label> 完整骨架，
; 演示 cs:ip 从 end start 的 start 标签处开始执行。
; 终态：ax = 000Ah（不通过内存，纯寄存器运算）。
code segment
    assume cs:code
start:
    mov ax, 0
    add ax, 3
    add ax, 3
    add ax, 4            ; 3 + 3 + 4 = 10 = 0Ah
    hlt
code ends
end start
