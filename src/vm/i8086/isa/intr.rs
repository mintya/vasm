//! 中断与中断屏蔽指令：`int / iret / cli / sti`。
//!
//! 分派语义（教学版）：
//! - **已知号** (21h/10h/16h) → 直接调 DOS/BIOS stub，不压栈、不动 IF。
//! - **未知号** → 读真实 IVT (`0:n*4`)：
//!   - 若向量全 0 → `UnhandledInterrupt`
//!   - 否则按 8086 标准：push flags → push cs → push ip → 清 IF/TF → 跳处理程序。
//!     处理程序末尾的 `iret` 才负责弹栈还原。
//!
//! 这种拆分让 stub 简洁（不必管栈），同时教材 §12.x "用户自定义中断 + iret" 全语义保留。

use crate::asm::ast::Operand;
use crate::asm::diagnostics::Span;
use crate::vm::bios;
use crate::vm::dos;
use crate::vm::i8086::cpu::Flags;
use crate::vm::i8086::exec::{Vm, VmError};
use crate::vm::i8086::isa::control::{pop_word, push_word};
use crate::vm::i8086::isa::operand::{OpSize, read_operand};

pub fn int_(vm: &mut Vm, ops: &[Operand], span: Span) -> Result<(), VmError> {
    if ops.len() != 1 {
        return Err(VmError::InvalidOperand {
            reason: format!("int expects 1 operand, got {}", ops.len()),
            span,
        });
    }
    let n = read_operand(vm, &ops[0], OpSize::Byte, span)? as u8;
    vm.console.bump_interrupts();

    match n {
        0x21 => dos::int21(vm, span),
        0x10 => bios::int10(vm, span),
        0x16 => bios::int16(vm, span),
        _ => dispatch_vector(vm, n, span),
    }
}

pub fn iret_(vm: &mut Vm, ops: &[Operand], span: Span) -> Result<(), VmError> {
    if !ops.is_empty() {
        return Err(VmError::InvalidOperand {
            reason: format!("iret expects 0 operands, got {}", ops.len()),
            span,
        });
    }
    let ip = pop_word(vm, span)?;
    let cs = pop_word(vm, span)?;
    let flags = pop_word(vm, span)?;
    vm.set_ip(ip);
    vm.cpu.cs = cs;
    vm.cpu.flags = Flags::from_u16(flags);
    Ok(())
}

pub fn cli_(vm: &mut Vm) -> Result<(), VmError> {
    vm.cpu.flags.if_ = false;
    Ok(())
}

pub fn sti_(vm: &mut Vm) -> Result<(), VmError> {
    vm.cpu.flags.if_ = true;
    Ok(())
}

fn dispatch_vector(vm: &mut Vm, n: u8, span: Span) -> Result<(), VmError> {
    let vec_phys = (n as u32) * 4;
    let target_ip = vm.mem.read_u16(vec_phys)?;
    let target_cs = vm.mem.read_u16(vec_phys + 2)?;
    if target_cs == 0 && target_ip == 0 {
        return Err(VmError::UnhandledInterrupt { num: n, span });
    }
    push_word(vm, vm.cpu.flags.to_u16(), span)?;
    push_word(vm, vm.cpu.cs, span)?;
    push_word(vm, vm.cpu.ip, span)?;
    vm.cpu.flags.if_ = false;
    vm.cpu.flags.tf = false;
    vm.cpu.cs = target_cs;
    vm.set_ip(target_ip);
    Ok(())
}
