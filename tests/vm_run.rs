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
    let mut vm = boot_str("code segment\n  in al, 60h\n  hlt\ncode ends\nend\n");
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
