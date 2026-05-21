; 教材 §7 fixture：灵活定位内存（[bx+si]、[bx+si+disp]）。
; 教学点：复合寻址形式，si/bx 同时参与有效地址计算。
; 数据段两行 4 字节"小矩阵"，第二行第三个字节 = 'G'（47h）。
; 终态：al = 47h ('G')。
data segment
    grid db 'A', 'B', 'C', 'D'      ; row 0
         db 'E', 'F', 'G', 'H'      ; row 1
data ends

code segment
    assume cs:code, ds:data
start:
    mov ax, data
    mov ds, ax
    mov bx, 4            ; 行基址：第 1 行从 offset 4 开始
    mov si, 2            ; 列偏移：第 2 列
    mov al, [bx+si]      ; grid[1][2] = 'G' = 47h
    hlt
code ends
end start
