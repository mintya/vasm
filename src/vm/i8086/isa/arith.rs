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

/// 与 sub 同语义但不写回结果，仅更新 flags。
pub fn cmp(vm: &mut Vm, ops: &[Operand], span: Span) -> Result<(), VmError> {
    expect_two(ops, "cmp", span)?;
    let size = infer_size(ops).unwrap_or(OpSize::Word);
    let a = read_operand(vm, &ops[0], size, span)?;
    let b = read_operand(vm, &ops[1], size, span)?;
    match size {
        OpSize::Byte => {
            let (ab, bb, rb) = (a as u8, b as u8, (a as u8).wrapping_sub(b as u8));
            flags::after_sub_u8(ab, bb, rb, &mut vm.cpu.flags);
        }
        OpSize::Word => {
            let r = a.wrapping_sub(b);
            flags::after_sub_u16(a, b, r, &mut vm.cpu.flags);
        }
    }
    Ok(())
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

/// `neg op`：result = 0 - op；CF = (op != 0)，OF/SF/ZF/AF/PF 按 sub。
pub fn neg(vm: &mut Vm, ops: &[Operand], span: Span) -> Result<(), VmError> {
    expect_one(ops, "neg", span)?;
    let size = infer_size(ops).unwrap_or(OpSize::Word);
    let a = read_operand(vm, &ops[0], size, span)?;
    let result = match size {
        OpSize::Byte => {
            let ab = a as u8;
            let r = 0u8.wrapping_sub(ab);
            flags::after_sub_u8(0, ab, r, &mut vm.cpu.flags);
            r as u16
        }
        OpSize::Word => {
            let r = 0u16.wrapping_sub(a);
            flags::after_sub_u16(0, a, r, &mut vm.cpu.flags);
            r
        }
    };
    write_operand(vm, &ops[0], size, result, span)
}

/// `mul src`：无符号乘法。
/// - byte：AX = AL × src8；CF=OF = (AH != 0)
/// - word：DX:AX = AX × src16；CF=OF = (DX != 0)
pub fn mul(vm: &mut Vm, ops: &[Operand], span: Span) -> Result<(), VmError> {
    expect_one(ops, "mul", span)?;
    let size = infer_size(ops).unwrap_or(OpSize::Word);
    let src = read_operand(vm, &ops[0], size, span)?;
    match size {
        OpSize::Byte => {
            let al = vm.cpu.ax as u8;
            let product = (al as u16).wrapping_mul((src as u8) as u16);
            vm.cpu.ax = product;
            let high_nonzero = (product >> 8) != 0;
            vm.cpu.flags.cf = high_nonzero;
            vm.cpu.flags.of = high_nonzero;
            // ZF/SF/PF undefined；按结果设置
            vm.cpu.flags.zf = product == 0;
            vm.cpu.flags.sf = product & 0x8000 != 0;
            vm.cpu.flags.pf = flags::parity(product as u8);
        }
        OpSize::Word => {
            let product = (vm.cpu.ax as u32).wrapping_mul(src as u32);
            vm.cpu.ax = product as u16;
            vm.cpu.dx = (product >> 16) as u16;
            let high_nonzero = vm.cpu.dx != 0;
            vm.cpu.flags.cf = high_nonzero;
            vm.cpu.flags.of = high_nonzero;
            vm.cpu.flags.zf = product == 0;
            vm.cpu.flags.sf = (vm.cpu.ax & 0x8000) != 0;
            vm.cpu.flags.pf = flags::parity(vm.cpu.ax as u8);
        }
    }
    Ok(())
}

/// `div src`：无符号除法。除零或商溢出抛 DivideByZero。
/// - byte：AL = AX / src8, AH = AX % src8
/// - word：AX = (DX:AX) / src16, DX = remainder
pub fn div(vm: &mut Vm, ops: &[Operand], span: Span) -> Result<(), VmError> {
    expect_one(ops, "div", span)?;
    let size = infer_size(ops).unwrap_or(OpSize::Word);
    let src = read_operand(vm, &ops[0], size, span)?;
    match size {
        OpSize::Byte => {
            let d = src as u8;
            if d == 0 {
                return Err(VmError::DivideByZero { span });
            }
            let dividend = vm.cpu.ax;
            let q = dividend / (d as u16);
            let r = dividend % (d as u16);
            if q > 0xFF {
                return Err(VmError::DivideByZero { span });
            }
            vm.cpu.ax = (r << 8) | q;
        }
        OpSize::Word => {
            if src == 0 {
                return Err(VmError::DivideByZero { span });
            }
            let dividend = ((vm.cpu.dx as u32) << 16) | (vm.cpu.ax as u32);
            let q = dividend / (src as u32);
            let r = dividend % (src as u32);
            if q > 0xFFFF {
                return Err(VmError::DivideByZero { span });
            }
            vm.cpu.ax = q as u16;
            vm.cpu.dx = r as u16;
        }
    }
    Ok(())
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
