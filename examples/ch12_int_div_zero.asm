; 教材 §12 fixture：内中断——除零（int 0）。
; 教学点：div 触发 DivideByZero VmError 后 VM 进 Error 状态，
; TUI 弹诊断浮层。headless 模式下 cargo test 会捕获错误。
; 期望：vm.run_until_halt 返 VmError::DivideByZero。
code segment
start:
    mov ax, 1
    mov dx, 0
    mov bx, 0            ; 除数 = 0
    div bx               ; 触发内中断 0（除零异常）
    hlt                  ; 不会到达
code ends
end start
