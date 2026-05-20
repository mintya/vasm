use crate::asm::ast::Operand;
use crate::asm::diagnostics::Span;
use crate::vm::i8086::exec::{Vm, VmError};
use crate::vm::i8086::isa::flags;
use crate::vm::i8086::isa::operand::{OpSize, infer_size, read_operand, write_operand};

pub fn add(vm: &mut Vm, ops: &[Operand], span: Span) -> Result<(), VmError> {
    expect_two(ops, "add", span)?;
    let size = infer_size(ops).unwrap_or(OpSize::Word);
    let a = read_operand(vm, &ops[0], size, span)?;
    let b = read_operand(vm, &ops[1], size, span)?;
    let result = match size {
        OpSize::Byte => {
            let (ab, bb, rb) = (a as u8, b as u8, (a as u8).wrapping_add(b as u8));
            flags::after_add_u8(ab, bb, rb, &mut vm.cpu.flags);
            rb as u16
        }
        OpSize::Word => {
            let r = a.wrapping_add(b);
            flags::after_add_u16(a, b, r, &mut vm.cpu.flags);
            r
        }
    };
    write_operand(vm, &ops[0], size, result, span)
}

pub fn sub(vm: &mut Vm, ops: &[Operand], span: Span) -> Result<(), VmError> {
    expect_two(ops, "sub", span)?;
    let size = infer_size(ops).unwrap_or(OpSize::Word);
    let a = read_operand(vm, &ops[0], size, span)?;
    let b = read_operand(vm, &ops[1], size, span)?;
    let result = match size {
        OpSize::Byte => {
            let (ab, bb, rb) = (a as u8, b as u8, (a as u8).wrapping_sub(b as u8));
            flags::after_sub_u8(ab, bb, rb, &mut vm.cpu.flags);
            rb as u16
        }
        OpSize::Word => {
            let r = a.wrapping_sub(b);
            flags::after_sub_u16(a, b, r, &mut vm.cpu.flags);
            r
        }
    };
    write_operand(vm, &ops[0], size, result, span)
}

pub fn inc(vm: &mut Vm, ops: &[Operand], span: Span) -> Result<(), VmError> {
    expect_one(ops, "inc", span)?;
    let size = infer_size(ops).unwrap_or(OpSize::Word);
    let a = read_operand(vm, &ops[0], size, span)?;
    let result = match size {
        OpSize::Byte => {
            let r = (a as u8).wrapping_add(1);
            flags::after_inc_u8(a as u8, r, &mut vm.cpu.flags);
            r as u16
        }
        OpSize::Word => {
            let r = a.wrapping_add(1);
            flags::after_inc_u16(a, r, &mut vm.cpu.flags);
            r
        }
    };
    write_operand(vm, &ops[0], size, result, span)
}

pub fn dec(vm: &mut Vm, ops: &[Operand], span: Span) -> Result<(), VmError> {
    expect_one(ops, "dec", span)?;
    let size = infer_size(ops).unwrap_or(OpSize::Word);
    let a = read_operand(vm, &ops[0], size, span)?;
    let result = match size {
        OpSize::Byte => {
            let r = (a as u8).wrapping_sub(1);
            flags::after_dec_u8(a as u8, r, &mut vm.cpu.flags);
            r as u16
        }
        OpSize::Word => {
            let r = a.wrapping_sub(1);
            flags::after_dec_u16(a, r, &mut vm.cpu.flags);
            r
        }
    };
    write_operand(vm, &ops[0], size, result, span)
}

fn expect_two(ops: &[Operand], name: &str, span: Span) -> Result<(), VmError> {
    if ops.len() != 2 {
        return Err(VmError::InvalidOperand {
            reason: format!("{name} expects 2 operands, got {}", ops.len()),
            span,
        });
    }
    Ok(())
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
