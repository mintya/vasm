; M1 fixture: 教材 §9 §10 §11 跳转、调用、条件
code segment
start:
    mov ax, 1
    cmp ax, 0
    je equal_label
    jne notequal_label
    jg greater_label
    jl less_label
    ja above_label
    jb below_label
    jcxz cxzero_label
    jmp short start
    jmp near ptr start
    call subproc
    ret
equal_label:
notequal_label:
greater_label:
less_label:
above_label:
below_label:
cxzero_label:
    nop
subproc:
    push ax
    pop ax
    ret
code ends
end start
