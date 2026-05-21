; 教材 §2 fixture：第一组寄存器操作。
; 教学点：mov 立即数 / 寄存器间复制 / add / hlt；不涉及内存。
; 终态：ax = 0009h, bx = 0003h（ax = 3 + bx，bx = 3）。
code segment
start:
    mov ax, 0001h
    mov bx, 0003h
    add ax, bx           ; ax = 4
    add ax, bx           ; ax = 7
    inc ax               ; ax = 8
    inc ax               ; ax = 9
    hlt
code ends
end start
