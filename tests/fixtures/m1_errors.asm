; M1 fixture: 各类错误，期望诊断快照
code segment
    mov ax,, bx          ; 多余逗号
    mov ax, [bx+bp]      ; 两个 base
    mov ax, [si+di]      ; 两个 index
    mov ax, []           ; 空内存操作数
    word ptr ax          ; size override 用在寄存器（这一行 word 被当成助记符，无错；下行才触发）
    mov ax, word ptr bx  ; size override on register
    mov ax, 12xy         ; 非法数字
    mov ax, 'AB' + 1     ; 多字节字符串在表达式中
    mov ax, ds:          ; seg override 后缺操作数
    mov ax, 'oops        ; 未闭合字符串
code ends
b ends                   ; ends 名字不匹配
end
