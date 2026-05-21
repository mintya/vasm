; 教材 §14 fixture：端口 I/O。
; 教学点：in al, port 从端口读字节；端口 60h 是键盘控制器扫描码（PS/2）。
; 教学场景下用 cargo test 先 vm.console.push_input(0x1E) 模拟扫描码，
; 程序 in 之后 al = 1Eh。
code segment
start:
    in  al, 60h          ; 读键盘扫描码（vm 把 console.pop_input 接到 60h 端口）
    hlt
code ends
end start
