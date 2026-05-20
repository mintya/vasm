; M5 fixture：单字符回显 echo。
; 循环用 int 21h ah=01h 阻塞读一个字符（自动回显到 console），
; 按 'q' 退出。这是验证 WaitingForInput 协议的最小程序。
code segment
    assume cs:code
start:
loop_top:
    mov ah, 1
    int 21h         ; al = 输入字符，并已回显
    cmp al, 'q'
    jne loop_top
    mov ah, 4ch
    int 21h
code ends
end start
