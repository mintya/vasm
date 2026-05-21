; 教材 §17 fixture：启动扇区读取。
; 用 int 13h ah=02h 把磁盘扇区 0（CHS = 0,0,1）读到 0:7C00（DOS 启动扇区惯例）。
; 配合 --disk 1.44MB 镜像跑：cargo run -- --disk floppy.img examples/ch17_disk_boot.asm
code segment
    assume cs:code
start:
    mov ax, 07C0h
    mov es, ax              ; 缓冲区段 = 07C0h
    mov bx, 0               ; 缓冲区偏移
    mov ah, 2               ; 功能号 02h：读扇区
    mov al, 1               ; 读 1 个扇区
    mov ch, 0               ; 柱面 0
    mov cl, 1               ; 扇区号 1（CHS 中扇区是 1-based）
    mov dh, 0               ; 磁头 0
    mov dl, 0               ; 驱动器 0（A:）
    int 13h
    ; 读完后 ax=0（成功）或 ax=错误码。停机。
    hlt
code ends
end start
