; M1 fixture: db/dw/dd 全形式
data segment
    a db 1
    b db 1, 2, 3
    c db 'A'
    s db 'hello$'
    w dw 1234h
    arr dw 1, 2, 3, 0aabbh
    big dd 12345678h
    pad db 16 dup (0)
    pad2 db 5 dup (1, 2, 3)
    nested db 3 dup (2 dup (0ffh))
    uninit dw ?
    uninit_arr db 4 dup (?)
data ends
end
