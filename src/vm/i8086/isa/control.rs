use crate::asm::ast::Operand;
use crate::asm::diagnostics::Span;
use crate::vm::i8086::cpu::Flags;
use crate::vm::i8086::exec::{Vm, VmError};
use crate::vm::i8086::isa::operand::{OpSize, read_operand};
use crate::vm::i8086::memory::Memory;

pub fn loop_(vm: &mut Vm, ops: &[Operand], span: Span) -> Result<(), VmError> {
    expect_one(ops, "loop", span)?;
    let target = read_operand(vm, &ops[0], OpSize::Word, span)?;
    let new_cx = vm.cpu.cx.wrapping_sub(1);
    vm.cpu.cx = new_cx;
    if new_cx != 0 {
        vm.set_ip(target);
    }
    Ok(())
}

pub fn hlt(vm: &mut Vm) -> Result<(), VmError> {
    vm.halt();
    Ok(())
}

pub fn nop() -> Result<(), VmError> {
    Ok(())
}

/// `jmp short/near label`：同段跳转。`jmp far ptr seg:off`：跨段。
/// `jmp word ptr ds:[...]` 等间接跳转留到 M6。
pub fn jmp(vm: &mut Vm, ops: &[Operand], span: Span) -> Result<(), VmError> {
    expect_one(ops, "jmp", span)?;
    match &ops[0] {
        Operand::Far { seg, off } => {
            vm.cpu.cs = *seg;
            vm.set_ip(*off);
            Ok(())
        }
        Operand::Imm(_) => {
            let target = read_operand(vm, &ops[0], OpSize::Word, span)?;
            vm.set_ip(target);
            Ok(())
        }
        Operand::Mem(_) => Err(VmError::UnsupportedInstruction {
            mnemonic: "jmp <mem>".into(),
            span,
        }),
        Operand::Reg(_) => Err(VmError::InvalidOperand {
            reason: "jmp 目标不能是寄存器（M6 才支持 jmp reg 间接跳转）".into(),
            span,
        }),
    }
}

/// 条件跳转：先按助记符确定条件，再 jmp 到目标。
pub fn jcc(vm: &mut Vm, mnemonic: &str, ops: &[Operand], span: Span) -> Result<(), VmError> {
    expect_one(ops, mnemonic, span)?;
    let take =
        jcc_condition(mnemonic, &vm.cpu.flags).ok_or_else(|| VmError::UnsupportedInstruction {
            mnemonic: mnemonic.into(),
            span,
        })?;
    if take {
        let target = read_operand(vm, &ops[0], OpSize::Word, span)?;
        vm.set_ip(target);
    }
    Ok(())
}

pub fn jcxz(vm: &mut Vm, ops: &[Operand], span: Span) -> Result<(), VmError> {
    expect_one(ops, "jcxz", span)?;
    if vm.cpu.cx == 0 {
        let target = read_operand(vm, &ops[0], OpSize::Word, span)?;
        vm.set_ip(target);
    }
    Ok(())
}

fn jcc_condition(mnemonic: &str, f: &Flags) -> Option<bool> {
    Some(match mnemonic {
        "je" | "jz" => f.zf,
        "jne" | "jnz" => !f.zf,
        "js" => f.sf,
        "jns" => !f.sf,
        "jo" => f.of,
        "jno" => !f.of,
        "jp" | "jpe" => f.pf,
        "jnp" | "jpo" => !f.pf,
        "jc" | "jb" | "jnae" => f.cf,
        "jnc" | "jae" | "jnb" => !f.cf,
        "jbe" | "jna" => f.cf || f.zf,
        "ja" | "jnbe" => !f.cf && !f.zf,
        "jl" | "jnge" => f.sf != f.of,
        "jge" | "jnl" => f.sf == f.of,
        "jle" | "jng" => f.zf || (f.sf != f.of),
        "jg" | "jnle" => !f.zf && (f.sf == f.of),
        _ => return None,
    })
}

/// `call near label` → push next_ip, ip = target
/// `call far seg:off` → push cs, push ip, set cs:ip
pub fn call(vm: &mut Vm, ops: &[Operand], span: Span) -> Result<(), VmError> {
    expect_one(ops, "call", span)?;
    match &ops[0] {
        Operand::Far { seg, off } => {
            push_word(vm, vm.cpu.cs, span)?;
            push_word(vm, vm.cpu.ip, span)?;
            vm.cpu.cs = *seg;
            vm.set_ip(*off);
            Ok(())
        }
        Operand::Imm(_) => {
            let target = read_operand(vm, &ops[0], OpSize::Word, span)?;
            push_word(vm, vm.cpu.ip, span)?;
            vm.set_ip(target);
            Ok(())
        }
        Operand::Mem(_) => Err(VmError::UnsupportedInstruction {
            mnemonic: "call <mem>".into(),
            span,
        }),
        Operand::Reg(_) => Err(VmError::InvalidOperand {
            reason: "call 不接受寄存器目标".into(),
            span,
        }),
    }
}

/// `ret`：pop ip。`ret imm16`：pop ip 后 sp += imm16。
pub fn ret(vm: &mut Vm, ops: &[Operand], span: Span) -> Result<(), VmError> {
    let new_ip = pop_word(vm, span)?;
    vm.set_ip(new_ip);
    if let [op] = ops {
        let extra = read_operand(vm, op, OpSize::Word, span)?;
        vm.cpu.sp = vm.cpu.sp.wrapping_add(extra);
    } else if ops.len() > 1 {
        return Err(VmError::InvalidOperand {
            reason: format!("ret expects 0 or 1 operand, got {}", ops.len()),
            span,
        });
    }
    Ok(())
}

/// `retf`：pop ip 后 pop cs。`retf imm16` 类似 ret imm。
pub fn retf(vm: &mut Vm, ops: &[Operand], span: Span) -> Result<(), VmError> {
    let new_ip = pop_word(vm, span)?;
    let new_cs = pop_word(vm, span)?;
    vm.set_ip(new_ip);
    vm.cpu.cs = new_cs;
    if let [op] = ops {
        let extra = read_operand(vm, op, OpSize::Word, span)?;
        vm.cpu.sp = vm.cpu.sp.wrapping_add(extra);
    } else if ops.len() > 1 {
        return Err(VmError::InvalidOperand {
            reason: format!("retf expects 0 or 1 operand, got {}", ops.len()),
            span,
        });
    }
    Ok(())
}

fn push_word(vm: &mut Vm, value: u16, _span: Span) -> Result<(), VmError> {
    vm.cpu.sp = vm.cpu.sp.wrapping_sub(2);
    let phys = Memory::phys(vm.cpu.ss, vm.cpu.sp);
    vm.mem.write_u16(phys, value)?;
    Ok(())
}

fn pop_word(vm: &mut Vm, _span: Span) -> Result<u16, VmError> {
    let phys = Memory::phys(vm.cpu.ss, vm.cpu.sp);
    let v = vm.mem.read_u16(phys)?;
    vm.cpu.sp = vm.cpu.sp.wrapping_add(2);
    Ok(v)
}

fn expect_one(ops: &[Operand], name: &str, span: Span) -> Result<(), VmError> {
    if ops.len() != 1 {
        return Err(VmError::InvalidOperand {
            reason: format!("{name} expects 1 operand, got {}", ops.len()),
            span,
        });
    }
    Ok(())
}
