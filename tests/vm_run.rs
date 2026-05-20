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
    let mut vm = boot_str("code segment\n  jmp far_away\n  hlt\ncode ends\nend\n");
    let err = vm.step().unwrap_err();
    assert!(matches!(err, VmError::UnsupportedInstruction { .. }));
}
