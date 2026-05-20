; M1 fixture: 教材 §3 §7 寻址方式
code segment
    mov ax, [bx]
    mov ax, [bx+si]
    mov ax, [bx+di]
    mov ax, [bp+si]
    mov ax, [bp+di]
    mov ax, [bx+5]
    mov ax, [bx+si+5]
    mov ax, [bx-3]
    mov ax, [bx+si-2+10]
    mov ax, ds:[bx]
    mov ax, es:[bx+si]
    mov ax, ss:[bp]
    mov ax, byte ptr [bx]
    mov ax, word ptr ds:[bx+si]
    mov ax, dword ptr [bx+5]
code ends
end
