use std::path::Path;

use vasm::asm::parser::parse;
use vasm::vm::i8086::exec::{Vm, VmError};
use vasm::vm::i8086::memory::Memory;

fn boot_str(src: &str) -> Vm {
    let (prog, diags) = parse(src);
    assert!(diags.is_empty(), "parse diags: {diags:?}");
    Vm::boot(prog, 1024).expect("boot")
}

fn run_fixture(name: &str) -> Vm {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(name);
    let src = std::fs::read_to_string(&path).expect("read fixture");
    let mut vm = boot_str(&src);
    vm.run_until_halt(100_000).expect("run");
    vm
}

#[test]
fn sum_loop_1_to_10_equals_55() {
    let vm = run_fixture("m2_sum_loop.asm");
    assert_eq!(vm.cpu.ax, 55);
    assert_eq!(vm.cpu.cx, 0);
}

#[test]
fn multi_segment_loads_segments_and_array_sum() {
    let vm = run_fixture("m2_multi_segment.asm");
    let data_seg = vm.program.segments["data"].base_paragraph;
    let stack_seg = vm.program.segments["stack"].base_paragraph;
    assert_eq!(vm.cpu.ds, data_seg);
    assert_eq!(vm.cpu.ss, stack_seg);
    assert_eq!(vm.cpu.ax, 6);
    assert_eq!(vm.cpu.bx, 6);
}

// ---- 细颗粒指令测试 -------------------------------------------------------

#[test]
fn mov_immediate_to_register() {
    let mut vm = boot_str("code segment\n  mov ax, 1234h\n  hlt\ncode ends\nend\n");
    vm.run_until_halt(100).unwrap();
    assert_eq!(vm.cpu.ax, 0x1234);
}

#[test]
fn mov_register_to_register_8bit() {
    let mut vm = boot_str("code segment\n  mov al, 0aah\n  mov bl, al\n  hlt\ncode ends\nend\n");
    vm.run_until_halt(100).unwrap();
    assert_eq!(vm.cpu.bx & 0xFF, 0xAA);
    assert_eq!(vm.cpu.ax & 0xFF, 0xAA);
}

#[test]
fn mov_immediate_to_segment_register_errors() {
    let mut vm = boot_str("code segment\n  mov ds, 1234h\n  hlt\ncode ends\nend\n");
    let err = vm.step().unwrap_err();
    assert!(matches!(err, VmError::SegRegImmediate { .. }));
}

#[test]
fn xchg_swaps_two_registers() {
    let mut vm =
        boot_str("code segment\n  mov ax, 1\n  mov bx, 2\n  xchg ax, bx\n  hlt\ncode ends\nend\n");
    vm.run_until_halt(100).unwrap();
    assert_eq!(vm.cpu.ax, 2);
    assert_eq!(vm.cpu.bx, 1);
}

#[test]
fn push_decrements_sp_and_writes_word() {
    let mut vm = boot_str(
        "stk segment\n  db 16 dup (0)\nstk ends\n\
         code segment\n  mov ax, stk\n  mov ss, ax\n  mov sp, 16\n  \
         mov ax, 0beefh\n  push ax\n  hlt\ncode ends\nend\n",
    );
    vm.run_until_halt(100).unwrap();
    assert_eq!(vm.cpu.sp, 14);
    let stk = vm.program.segments["stk"].base_paragraph;
    let phys = Memory::phys(stk, 14);
    assert_eq!(vm.mem.read_u16(phys).unwrap(), 0xBEEF);
}

#[test]
fn pop_increments_sp_and_reads_word() {
    let mut vm = boot_str(
        "stk segment\n  db 16 dup (0)\nstk ends\n\
         code segment\n  mov ax, stk\n  mov ss, ax\n  mov sp, 16\n  \
         mov ax, 0cafeh\n  push ax\n  pop bx\n  hlt\ncode ends\nend\n",
    );
    vm.run_until_halt(100).unwrap();
    assert_eq!(vm.cpu.bx, 0xCAFE);
    assert_eq!(vm.cpu.sp, 16);
}

#[test]
fn add_sets_zf_on_zero_result() {
    let mut vm = boot_str("code segment\n  mov ax, 0ffffh\n  add ax, 1\n  hlt\ncode ends\nend\n");
    vm.run_until_halt(100).unwrap();
    assert_eq!(vm.cpu.ax, 0);
    assert!(vm.cpu.flags.zf);
    assert!(vm.cpu.flags.cf);
    assert!(!vm.cpu.flags.of);
}

#[test]
fn add_signed_overflow_sets_of() {
    let mut vm = boot_str("code segment\n  mov ax, 7fffh\n  add ax, 1\n  hlt\ncode ends\nend\n");
    vm.run_until_halt(100).unwrap();
    assert_eq!(vm.cpu.ax, 0x8000);
    assert!(vm.cpu.flags.of);
    assert!(vm.cpu.flags.sf);
    assert!(!vm.cpu.flags.cf);
}

#[test]
fn sub_borrow_sets_cf() {
    let mut vm = boot_str("code segment\n  mov ax, 1\n  sub ax, 2\n  hlt\ncode ends\nend\n");
    vm.run_until_halt(100).unwrap();
    assert_eq!(vm.cpu.ax, 0xFFFF);
    assert!(vm.cpu.flags.cf);
    assert!(vm.cpu.flags.sf);
}

#[test]
fn inc_does_not_change_cf() {
    let mut vm =
        boot_str("code segment\n  mov ax, 1\n  add ax, 0ffffh\n  inc bx\n  hlt\ncode ends\nend\n");
    vm.run_until_halt(100).unwrap();
    // add 让 cf=1，inc 不应改 cf
    assert!(vm.cpu.flags.cf);
    assert_eq!(vm.cpu.bx, 1);
}

#[test]
fn loop_decrements_cx_and_jumps() {
    let mut vm =
        boot_str("code segment\n  mov cx, 5\nl:\n  inc bx\n  loop l\n  hlt\ncode ends\nend\n");
    vm.run_until_halt(1000).unwrap();
    assert_eq!(vm.cpu.bx, 5);
    assert_eq!(vm.cpu.cx, 0);
}

#[test]
fn pushf_popf_round_trip() {
    let mut vm = boot_str(
        "stk segment\n  db 16 dup (0)\nstk ends\n\
         code segment\n  mov ax, stk\n  mov ss, ax\n  mov sp, 16\n  \
         mov ax, 1\n  add ax, 0ffffh\n  pushf\n  mov ax, 5\n  popf\n  hlt\ncode ends\nend\n",
    );
    vm.run_until_halt(1000).unwrap();
    // 进 popf 后 flags 应恢复到 add 之后的状态：ZF=1 CF=1
    assert!(vm.cpu.flags.zf);
    assert!(vm.cpu.flags.cf);
}

#[test]
fn fall_off_segment_end_halts() {
    let mut vm = boot_str("code segment\n  mov ax, 1\ncode ends\nend\n");
    vm.run_until_halt(100).unwrap();
    assert_eq!(vm.cpu.ax, 1);
    assert!(vm.halted());
}

#[test]
fn unsupported_instruction_reports_error() {
    // 选一个真没接入的助记符（lea 教材后期才用，仓库未实现）
    let mut vm = boot_str("code segment\n  lea ax, [bx]\n  hlt\ncode ends\nend\n");
    let err = vm.step().unwrap_err();
    assert!(matches!(err, VmError::UnsupportedInstruction { .. }));
}

// -------------------- M4 端到端 --------------------

#[test]
fn cmp_jne_loop_counts_down_to_zero() {
    // 等价于 sum_loop 但用 cmp+jne 控制循环
    let mut vm = boot_str(
        "code segment\nstart:\n  mov ax, 0\n  mov bx, 10\nlp:\n  add ax, bx\n  dec bx\n  cmp bx, 0\n  jne lp\n  hlt\ncode ends\nend start\n",
    );
    vm.run_until_halt(1000).unwrap();
    assert_eq!(vm.cpu.ax, 55); // 10+9+...+1
    assert_eq!(vm.cpu.bx, 0);
    assert!(vm.cpu.flags.zf);
}

#[test]
fn call_ret_returns_to_next_instruction() {
    let mut vm = boot_str(
        "stack segment\n  db 32 dup (0)\nstack ends\ncode segment\n  assume cs:code, ss:stack\nstart:\n  mov ax, stack\n  mov ss, ax\n  mov sp, 32\n  call inc_ax\n  call inc_ax\n  hlt\ninc_ax:\n  inc ax\n  ret\ncode ends\nend start\n",
    );
    vm.run_until_halt(100).unwrap();
    assert_eq!(vm.cpu.ax & 0xFF, 2); // 调用两次 inc
    assert_eq!(vm.cpu.sp, 32); // 栈对称回到初始
}

#[test]
fn shl_by_four_multiplies_by_sixteen() {
    let mut vm = boot_str("code segment\n  mov ax, 3\n  shl ax, 4\n  hlt\ncode ends\nend\n");
    vm.run_until_halt(100).unwrap();
    assert_eq!(vm.cpu.ax, 0x0030);
}

#[test]
fn mul_word_writes_dx_ax() {
    let mut vm =
        boot_str("code segment\n  mov ax, 1000h\n  mov bx, 10h\n  mul bx\n  hlt\ncode ends\nend\n");
    vm.run_until_halt(100).unwrap();
    // 0x1000 * 0x10 = 0x10000 → dx=1 ax=0
    assert_eq!(vm.cpu.dx, 1);
    assert_eq!(vm.cpu.ax, 0);
    assert!(vm.cpu.flags.cf); // dx 非零
}

#[test]
fn div_word_quotient_and_remainder() {
    let mut vm = boot_str(
        "code segment\n  mov dx, 0\n  mov ax, 100\n  mov bx, 7\n  div bx\n  hlt\ncode ends\nend\n",
    );
    vm.run_until_halt(100).unwrap();
    assert_eq!(vm.cpu.ax, 14); // 100/7
    assert_eq!(vm.cpu.dx, 2); // 100%7
}

#[test]
fn div_by_zero_returns_error() {
    let mut vm = boot_str(
        "code segment\n  mov dx, 0\n  mov ax, 1\n  mov bx, 0\n  div bx\n  hlt\ncode ends\nend\n",
    );
    let err = vm.run_until_halt(100).unwrap_err();
    assert!(matches!(err, VmError::DivideByZero { .. }));
}

#[test]
fn jcxz_takes_branch_when_cx_zero() {
    let mut vm = boot_str(
        "code segment\nstart:\n  mov cx, 0\n  jcxz skip\n  mov ax, 1\nskip:\n  mov bx, 2\n  hlt\ncode ends\nend start\n",
    );
    vm.run_until_halt(100).unwrap();
    assert_eq!(vm.cpu.ax, 0); // 没执行
    assert_eq!(vm.cpu.bx, 2);
}

#[test]
fn xor_clears_register_and_sets_zf() {
    let mut vm = boot_str("code segment\n  mov ax, 0FFFFh\n  xor ax, ax\n  hlt\ncode ends\nend\n");
    vm.run_until_halt(100).unwrap();
    assert_eq!(vm.cpu.ax, 0);
    assert!(vm.cpu.flags.zf);
}

#[test]
fn unconditional_jmp_to_label() {
    let mut vm = boot_str(
        "code segment\nstart:\n  jmp forward\n  mov ax, 1\nforward:\n  mov ax, 42\n  hlt\ncode ends\nend start\n",
    );
    vm.run_until_halt(100).unwrap();
    assert_eq!(vm.cpu.ax, 42);
}

// ---- M5：中断与 I/O ------------------------------------------------------

#[test]
fn dos_02h_writes_char_to_console() {
    let mut vm = boot_str(
        "code segment\n  mov dl, 'X'\n  mov ah, 2\n  int 21h\n  mov ah, 4ch\n  int 21h\ncode ends\nend\n",
    );
    vm.run_until_halt(100).unwrap();
    assert_eq!(vm.console.output(), b"X");
    assert!(vm.halted());
}

#[test]
fn dos_09h_writes_dollar_terminated_string() {
    let mut vm = boot_str(
        "data segment\n  msg db 'Hello,$junk'\ndata ends\n\
         code segment\n  assume cs:code, ds:data\nstart:\n  mov ax, data\n  mov ds, ax\n  \
         mov dx, offset msg\n  mov ah, 9\n  int 21h\n  mov ah, 4ch\n  int 21h\n\
         code ends\nend start\n",
    );
    vm.run_until_halt(200).unwrap();
    assert_eq!(vm.console.output(), b"Hello,");
}

#[test]
fn dos_4ch_halts() {
    let mut vm = boot_str("code segment\n  mov ah, 4ch\n  int 21h\ncode ends\nend\n");
    vm.run_until_halt(100).unwrap();
    assert!(vm.halted());
}

#[test]
fn unsupported_dos_func_returns_error() {
    let mut vm = boot_str("code segment\n  mov ah, 88h\n  int 21h\ncode ends\nend\n");
    let err = vm.run_until_halt(100).unwrap_err();
    assert!(matches!(err, VmError::UnsupportedDosFunc { ah: 0x88, .. }));
}

#[test]
fn bios_int10_09h_repeats_char() {
    let mut vm = boot_str(
        "code segment\n  mov al, '*'\n  mov cx, 5\n  mov ah, 9\n  int 10h\n  \
         mov ah, 4ch\n  int 21h\ncode ends\nend\n",
    );
    vm.run_until_halt(100).unwrap();
    assert_eq!(vm.console.output(), b"*****");
}

#[test]
fn int_unknown_with_empty_ivt_returns_error() {
    let mut vm = boot_str("code segment\n  int 99h\ncode ends\nend\n");
    let err = vm.run_until_halt(100).unwrap_err();
    assert!(matches!(err, VmError::UnhandledInterrupt { num: 0x99, .. }));
}

#[test]
fn user_int_via_ivt_then_iret_round_trip() {
    // 在 Rust 侧装好 IVT[7Ch]、栈，再让程序触发 int 7Ch；handler 改 bx 后 iret；
    // 验证：handler 跑到、ax 不被破坏、cpu/flags 完整还原。
    let mut vm = boot_str(
        "stack segment\n  db 64 dup (0)\nstack ends\n\
         code segment\n  assume cs:code, ss:stack\nstart:\n  \
         mov ax, stack\n  mov ss, ax\n  mov sp, 64\n  \
         mov ax, 0AAAAh\n  \
         int 7ch\n  \
         hlt\n\
         handler:\n  mov bx, 1234h\n  iret\n\
         code ends\nend start\n",
    );
    // 找 handler 的入口偏移，写到 IVT[7Ch]
    let code_seg = vm.program.segments["code"].base_paragraph;
    let handler_off = vm
        .program
        .symbols
        .get("handler")
        .expect("handler symbol")
        .offset;
    vm.mem.write_u16(0x7C * 4, handler_off).unwrap();
    vm.mem.write_u16(0x7C * 4 + 2, code_seg).unwrap();

    vm.run_until_halt(1000).unwrap();
    assert_eq!(vm.cpu.bx, 0x1234, "handler executed");
    assert_eq!(vm.cpu.ax, 0xAAAA, "ax preserved across int+iret");
    assert!(vm.halted());
}

#[test]
fn dos_01h_waits_then_consumes_input() {
    use vasm::vm::i8086::exec::StepOutcome;
    let mut vm = boot_str(
        "code segment\nstart:\n  mov ah, 1\n  int 21h\n  mov ah, 4ch\n  int 21h\ncode ends\nend start\n",
    );
    // 第一步 mov ah, 1 OK
    assert_eq!(vm.step().unwrap(), StepOutcome::Stepped);
    let ip_before = vm.cpu.ip;
    // 第二步：int 21h，无输入 → WaitingForInput，ip 退回
    assert_eq!(vm.step().unwrap(), StepOutcome::WaitingForInput);
    assert_eq!(vm.cpu.ip, ip_before, "ip rewound to int instruction");
    // 喂一个字节
    vm.console.push_input(b'k');
    // 再 step → 消费输入、al = 'k'、回显
    assert_eq!(vm.step().unwrap(), StepOutcome::Stepped);
    assert_eq!(vm.cpu.ax & 0xFF, b'k' as u16);
    assert_eq!(vm.console.output(), b"k");
}

#[test]
fn cli_sti_toggle_if_flag() {
    let mut vm = boot_str("code segment\n  sti\n  cli\n  sti\n  hlt\ncode ends\nend\n");
    use vasm::vm::i8086::exec::StepOutcome;
    assert_eq!(vm.step().unwrap(), StepOutcome::Stepped); // sti
    assert!(vm.cpu.flags.if_);
    assert_eq!(vm.step().unwrap(), StepOutcome::Stepped); // cli
    assert!(!vm.cpu.flags.if_);
    assert_eq!(vm.step().unwrap(), StepOutcome::Stepped); // sti
    assert!(vm.cpu.flags.if_);
}

#[test]
fn in_port_60h_pops_input_byte() {
    let mut vm = boot_str("code segment\n  in al, 60h\n  hlt\ncode ends\nend\n");
    vm.console.push_input(0x1Eu8); // 'a' 扫描码
    vm.run_until_halt(100).unwrap();
    assert_eq!(vm.cpu.ax & 0xFF, 0x1E);
}

#[test]
fn in_unsupported_port_returns_error() {
    let mut vm = boot_str("code segment\n  in al, 70h\n  hlt\ncode ends\nend\n");
    let err = vm.run_until_halt(100).unwrap_err();
    assert!(matches!(err, VmError::UnsupportedPort { port: 0x70, .. }));
}

#[test]
fn m5_hello_fixture_writes_string_to_console() {
    let vm = run_fixture("m5_hello.asm");
    assert_eq!(vm.console.output(), b"Hello, world!");
    assert!(vm.halted());
}

#[test]
fn m5_bios_video_fixture_repeats_char() {
    let vm = run_fixture("m5_bios_video.asm");
    assert_eq!(vm.console.output(), b"********************");
    assert!(vm.halted());
}

#[test]
fn dos_0ah_buffered_input_handles_backspace() {
    // 程序用 ah=0Ah 缓冲输入读入；预先在 vm.console.input 塞 "ab\x08c\r"。
    // 期望：缓冲区里只剩 "ac"，count=2；output 出现 a, b, \b\x20\b, c, \r\n。
    use vasm::vm::i8086::memory::Memory;

    let mut vm = boot_str(
        "data segment\n  buf db 16, 0, 16 dup (0)\ndata ends\n\
         code segment\n  assume cs:code, ds:data\nstart:\n  \
         mov ax, data\n  mov ds, ax\n  \
         mov dx, offset buf\n  mov ah, 0Ah\n  int 21h\n  \
         mov ah, 4ch\n  int 21h\ncode ends\nend start\n",
    );
    // 先 step 几步让 ds 设好（实际上 run_until_halt 之前要先入队）
    vm.console.push_input(b'a');
    vm.console.push_input(b'b');
    vm.console.push_input(0x08);
    vm.console.push_input(b'c');
    vm.console.push_input(b'\r');

    vm.run_until_halt(1000).unwrap();

    let ds = vm.cpu.ds;
    let count = vm.mem.read_u8(Memory::phys(ds, 1)).unwrap();
    assert_eq!(count, 2, "Backspace 应抵消一个字符");
    assert_eq!(vm.mem.read_u8(Memory::phys(ds, 2)).unwrap(), b'a');
    assert_eq!(vm.mem.read_u8(Memory::phys(ds, 3)).unwrap(), b'c');
    assert_eq!(
        vm.mem.read_u8(Memory::phys(ds, 4)).unwrap(),
        0x0D,
        "DOS 在缓冲末尾写 0x0D"
    );

    // output 应当包含 'a' 'b' \b\x20\b 'c' \r\n
    let out = vm.console.output();
    assert!(out.contains(&b'a') && out.contains(&b'b') && out.contains(&b'c'));
    assert!(
        out.windows(3).any(|w| w == [0x08, 0x20, 0x08]),
        "应有 08 20 08 视觉擦除序列: {out:?}"
    );
    assert!(out.ends_with(b"\r\n"), "应以 CRLF 收尾: {out:?}");
}

#[test]
fn dos_0ah_buffered_input_truncates_at_capacity() {
    // max=4 → 容量 3 字符。塞 "abcde\r" 只应保留 "abc"。
    use vasm::vm::i8086::memory::Memory;

    let mut vm = boot_str(
        "data segment\n  buf db 4, 0, 8 dup (0)\ndata ends\n\
         code segment\n  assume cs:code, ds:data\nstart:\n  \
         mov ax, data\n  mov ds, ax\n  \
         mov dx, offset buf\n  mov ah, 0Ah\n  int 21h\n  \
         mov ah, 4ch\n  int 21h\ncode ends\nend start\n",
    );
    for b in b"abcde\r" {
        vm.console.push_input(*b);
    }
    vm.run_until_halt(1000).unwrap();
    let ds = vm.cpu.ds;
    let count = vm.mem.read_u8(Memory::phys(ds, 1)).unwrap();
    assert_eq!(count, 3);
    assert_eq!(vm.mem.read_u8(Memory::phys(ds, 2)).unwrap(), b'a');
    assert_eq!(vm.mem.read_u8(Memory::phys(ds, 3)).unwrap(), b'b');
    assert_eq!(vm.mem.read_u8(Memory::phys(ds, 4)).unwrap(), b'c');
}

// ---- M6 Stage A：间接跳转 + 磁盘 ----------------------------------------

#[test]
fn m6_jump_table_fixture_calls_correct_handler() {
    let vm = run_fixture("m6_jump_table.asm");
    // table[1] = h1 → ax = 200
    assert_eq!(vm.cpu.ax, 200);
}

#[test]
fn jmp_word_ptr_near_indirect() {
    let mut vm = boot_str(
        "data segment\n  tbl dw target\ndata ends\n\
         code segment\n  assume cs:code, ds:data\nstart:\n  \
         mov ax, data\n  mov ds, ax\n  mov bx, 0\n  \
         jmp word ptr ds:[bx + offset tbl]\n  \
         mov ax, 1\n  hlt\n\
         target:\n  mov ax, 0FACEh\n  hlt\n\
         code ends\nend start\n",
    );
    vm.run_until_halt(1000).unwrap();
    assert_eq!(vm.cpu.ax, 0xFACE);
}

#[test]
fn call_word_ptr_indirect_pushes_return_ip() {
    let mut vm = boot_str(
        "data segment\n  tbl dw sub_routine\ndata ends\n\
         stack segment\n  db 32 dup (0)\nstack ends\n\
         code segment\n  assume cs:code, ds:data, ss:stack\nstart:\n  \
         mov ax, stack\n  mov ss, ax\n  mov sp, 32\n  \
         mov ax, data\n  mov ds, ax\n  mov bx, 0\n  \
         call word ptr ds:[bx + offset tbl]\n  \
         mov ax, 0BEEFh\n  hlt\n\
         sub_routine:\n  mov cx, 1234h\n  ret\n\
         code ends\nend start\n",
    );
    vm.run_until_halt(1000).unwrap();
    assert_eq!(vm.cpu.cx, 0x1234, "sub_routine 跑过");
    assert_eq!(vm.cpu.ax, 0xBEEF, "ret 后回到 mov ax, 0BEEFh");
    assert_eq!(vm.cpu.sp, 32, "栈对称");
}

#[test]
fn jmp_reg_indirect() {
    let mut vm = boot_str(
        "code segment\nstart:\n  mov ax, offset target\n  jmp ax\n  \
         mov bx, 1\n  hlt\n\
         target:\n  mov bx, 0DEADh\n  hlt\n\
         code ends\nend start\n",
    );
    vm.run_until_halt(1000).unwrap();
    assert_eq!(vm.cpu.bx, 0xDEAD);
}

#[test]
fn int13_read_sector_writes_to_es_bx() {
    let mut vm = boot_str(
        "code segment\nstart:\n  \
         mov ax, 07C0h\n  mov es, ax\n  mov bx, 0\n  \
         mov ah, 2\n  mov al, 1\n  mov ch, 0\n  mov cl, 1\n  mov dh, 0\n  mov dl, 0\n  \
         int 13h\n  hlt\ncode ends\nend start\n",
    );
    // 准备一张磁盘：扇区 0 的前 8 字节填 "BOOTSECT"
    let mut disk = vec![0u8; 1_474_560];
    disk[..8].copy_from_slice(b"BOOTSECT");
    vm.disk = Some(disk);

    vm.run_until_halt(1000).unwrap();
    assert_eq!(vm.cpu.r8(vasm::vm::i8086::cpu::Reg8::Ah), 0, "AH=0 表成功");
    assert!(!vm.cpu.flags.cf, "CF=0 表成功");
    // 读到的字节应在 07C0:0000 = 0x7C00
    use vasm::vm::i8086::memory::Memory;
    for (i, b) in b"BOOTSECT".iter().enumerate() {
        let v = vm.mem.read_u8(Memory::phys(0x07C0, i as u16)).unwrap();
        assert_eq!(v, *b, "byte {i}");
    }
}

#[test]
fn int13_without_disk_returns_error() {
    let mut vm = boot_str(
        "code segment\n  mov ah, 2\n  mov al, 1\n  mov ch, 0\n  mov cl, 1\n  \
         mov dh, 0\n  mov dl, 0\n  int 13h\n  hlt\ncode ends\nend\n",
    );
    let err = vm.run_until_halt(100).unwrap_err();
    assert!(matches!(err, VmError::DiskIo { .. }));
}

#[test]
fn int16_ah_02_returns_zero_flags_byte() {
    let mut vm = boot_str("code segment\n  mov ah, 2\n  int 16h\n  hlt\ncode ends\nend\n");
    vm.run_until_halt(100).unwrap();
    use vasm::vm::i8086::cpu::Reg8;
    assert_eq!(vm.cpu.r8(Reg8::Al), 0, "教学场景修饰键状态恒 0");
}

#[test]
fn int16_ah_11_behaves_like_01() {
    let mut vm = boot_str("code segment\n  mov ah, 11h\n  int 16h\n  hlt\ncode ends\nend\n");
    // 空缓冲：ZF=1
    vm.run_until_halt(100).unwrap();
    assert!(vm.cpu.flags.zf, "ah=11 空缓冲时 ZF=1");
}

// ---- M6 Stage B：undo + watchpoint -------------------------------------

#[test]
fn step_with_snapshot_captures_cpu_before() {
    let mut vm =
        boot_str("code segment\n  mov ax, 1234h\n  mov bx, 5678h\n  hlt\ncode ends\nend\n");
    let cpu0 = vm.cpu;
    let (_, snap) = vm.step_with_snapshot().unwrap();
    assert_eq!(snap.cpu_before, cpu0, "snapshot 应是 step 前的 cpu");
    assert_ne!(vm.cpu.ax, snap.cpu_before.ax, "step 后 ax 已变");
}

#[test]
fn step_with_snapshot_records_memory_diffs() {
    // push 一字到栈段：栈段被写了 2 字节
    let mut vm = boot_str(
        "stk segment\n  db 16 dup (0)\nstk ends\n\
         code segment\n  mov ax, stk\n  mov ss, ax\n  mov sp, 16\n  \
         mov ax, 0BEEFh\n  push ax\n  hlt\ncode ends\nend\n",
    );
    // 跑到 push 之前
    for _ in 0..4 {
        vm.step().unwrap();
    }
    let (_, snap) = vm.step_with_snapshot().unwrap();
    // push 写了 2 字节（u16 = lo + hi）→ mem_diffs 含 2 项 (addr, 旧值=0)
    assert_eq!(snap.mem_diffs.len(), 2);
    for (_, old) in &snap.mem_diffs {
        assert_eq!(*old, 0);
    }
}

#[test]
fn step_with_snapshot_records_console_output_len() {
    let mut vm =
        boot_str("code segment\n  mov dl, 'X'\n  mov ah, 2\n  int 21h\n  hlt\ncode ends\nend\n");
    // 走到 int 21h 之前
    vm.step().unwrap(); // mov dl
    vm.step().unwrap(); // mov ah
    let len_before = vm.console.output_len();
    let (_, snap) = vm.step_with_snapshot().unwrap();
    assert_eq!(snap.console_output_len_before, len_before);
    assert!(vm.console.output_len() > len_before, "int 21h 应输出字符");
}
