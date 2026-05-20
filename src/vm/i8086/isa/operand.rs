use crate::asm::ast::{BinOp, DataSize, Expr, Mem, Operand};
use crate::asm::diagnostics::Span;
use crate::vm::i8086::cpu::{Reg16, RegRef};
use crate::vm::i8086::exec::{Vm, VmError};
use crate::vm::i8086::loader::SymbolKind;
use crate::vm::i8086::memory::Memory;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpSize {
    Byte,
    Word,
}

pub fn infer_size(operands: &[Operand]) -> Option<OpSize> {
    for op in operands {
        match op {
            Operand::Reg(name) => {
                return Some(if is_byte_register(name) {
                    OpSize::Byte
                } else {
                    OpSize::Word
                });
            }
            Operand::Mem(m) => match m.size {
                Some(DataSize::Byte) => return Some(OpSize::Byte),
                Some(DataSize::Word) => return Some(OpSize::Word),
                _ => {}
            },
            _ => {}
        }
    }
    None
}

pub fn is_byte_register(name: &str) -> bool {
    matches!(name, "al" | "ah" | "bl" | "bh" | "cl" | "ch" | "dl" | "dh")
}

pub fn is_seg_register(name: &str) -> bool {
    matches!(name, "cs" | "ds" | "ss" | "es")
}

pub fn read_operand(vm: &Vm, op: &Operand, size: OpSize, span: Span) -> Result<u16, VmError> {
    if let Some(mem) = imm_ident_to_data_mem(vm, op) {
        return read_memory(vm, &mem, size, span);
    }
    match op {
        Operand::Reg(name) => read_register(vm, name, size, span),
        Operand::Imm(expr) => Ok(eval_expr(vm, expr, span)? as u16),
        Operand::Mem(m) => read_memory(vm, m, size, span),
        Operand::Far { .. } => Err(VmError::InvalidOperand {
            reason: "far ptr literal not supported in M2".into(),
            span,
        }),
    }
}

pub fn write_operand(
    vm: &mut Vm,
    op: &Operand,
    size: OpSize,
    value: u16,
    span: Span,
) -> Result<(), VmError> {
    if let Some(mem) = imm_ident_to_data_mem(vm, op) {
        return write_memory(vm, &mem, size, value, span);
    }
    match op {
        Operand::Reg(name) => write_register(vm, name, size, value, span),
        Operand::Mem(m) => write_memory(vm, m, size, value, span),
        Operand::Imm(_) | Operand::Far { .. } => Err(VmError::InvalidOperand {
            reason: "cannot write to immediate operand".into(),
            span,
        }),
    }
}

pub fn effective_address(vm: &Vm, mem: &Mem, span: Span) -> Result<(u16, u16), VmError> {
    let base = match mem.base.as_deref() {
        Some(name) => reg16(name, span)?.map(|r| vm.cpu.r16(r)).unwrap_or(0),
        None => 0,
    };
    let index = match mem.index.as_deref() {
        Some(name) => reg16(name, span)?.map(|r| vm.cpu.r16(r)).unwrap_or(0),
        None => 0,
    };
    let disp = match &mem.disp {
        Some(expr) => eval_expr(vm, expr, span)? as u16,
        None => 0,
    };

    let offset = base.wrapping_add(index).wrapping_add(disp);

    let seg_reg = match &mem.seg_override {
        Some(name) => reg16(name, span)?.ok_or_else(|| VmError::InvalidOperand {
            reason: format!("`{name}` is not a 16-bit register"),
            span,
        })?,
        None => default_segment(mem),
    };
    Ok((vm.cpu.r16(seg_reg), offset))
}

pub fn eval_expr(vm: &Vm, expr: &Expr, span: Span) -> Result<i64, VmError> {
    match expr {
        Expr::Int(n) => Ok(*n),
        Expr::Neg(inner) => Ok(-eval_expr(vm, inner, span)?),
        Expr::BinOp { op, lhs, rhs } => {
            let l = eval_expr(vm, lhs, span)?;
            let r = eval_expr(vm, rhs, span)?;
            apply_binop(*op, l, r, span)
        }
        Expr::Ident(name) => resolve_ident(vm, name, span),
        Expr::Offset(inner) => match inner.as_ref() {
            Expr::Ident(name) => offset_of(vm, name, span),
            _ => eval_expr(vm, inner, span),
        },
        Expr::Seg(inner) => match inner.as_ref() {
            Expr::Ident(name) => seg_paragraph_of(vm, name, span),
            _ => Err(VmError::InvalidOperand {
                reason: "`seg` requires a label or segment name".into(),
                span,
            }),
        },
    }
}

fn apply_binop(op: BinOp, l: i64, r: i64, span: Span) -> Result<i64, VmError> {
    Ok(match op {
        BinOp::Add => l.wrapping_add(r),
        BinOp::Sub => l.wrapping_sub(r),
        BinOp::Mul => l.wrapping_mul(r),
        BinOp::Div => {
            if r == 0 {
                return Err(VmError::DivideByZero { span });
            }
            l.wrapping_div(r)
        }
        BinOp::Mod => {
            if r == 0 {
                return Err(VmError::DivideByZero { span });
            }
            l.wrapping_rem(r)
        }
    })
}

fn resolve_ident(vm: &Vm, name: &str, span: Span) -> Result<i64, VmError> {
    let sym = vm
        .program
        .symbols
        .get(name)
        .ok_or_else(|| VmError::UndefinedSymbol {
            name: name.into(),
            span,
        })?;
    match sym.kind {
        SymbolKind::SegmentName => Ok(vm.program.segments[&sym.segment].base_paragraph as i64),
        _ => Ok(sym.offset as i64),
    }
}

fn offset_of(vm: &Vm, name: &str, span: Span) -> Result<i64, VmError> {
    let sym = vm
        .program
        .symbols
        .get(name)
        .ok_or_else(|| VmError::UndefinedSymbol {
            name: name.into(),
            span,
        })?;
    Ok(sym.offset as i64)
}

fn seg_paragraph_of(vm: &Vm, name: &str, span: Span) -> Result<i64, VmError> {
    let sym = vm
        .program
        .symbols
        .get(name)
        .ok_or_else(|| VmError::UndefinedSymbol {
            name: name.into(),
            span,
        })?;
    Ok(vm.program.segments[&sym.segment].base_paragraph as i64)
}

fn imm_ident_to_data_mem(vm: &Vm, op: &Operand) -> Option<Mem> {
    if let Operand::Imm(Expr::Ident(name)) = op
        && let Some(sym) = vm.program.symbols.get(name)
        && matches!(
            sym.kind,
            SymbolKind::DataByte | SymbolKind::DataWord | SymbolKind::DataDword
        )
    {
        return Some(Mem {
            disp: Some(Expr::Ident(name.clone())),
            ..Mem::default()
        });
    }
    None
}

fn read_register(vm: &Vm, name: &str, size: OpSize, span: Span) -> Result<u16, VmError> {
    let r = RegRef::from_name(name).ok_or_else(|| VmError::InvalidOperand {
        reason: format!("unknown register `{name}`"),
        span,
    })?;
    match (r, size) {
        (RegRef::R16(r), OpSize::Word) => Ok(vm.cpu.r16(r)),
        (RegRef::R8(r), OpSize::Byte) => Ok(vm.cpu.r8(r) as u16),
        (RegRef::R16(_), OpSize::Byte) => Err(VmError::InvalidOperand {
            reason: format!("byte operation on 16-bit register `{name}`"),
            span,
        }),
        (RegRef::R8(_), OpSize::Word) => Err(VmError::InvalidOperand {
            reason: format!("word operation on 8-bit register `{name}`"),
            span,
        }),
    }
}

fn write_register(
    vm: &mut Vm,
    name: &str,
    size: OpSize,
    value: u16,
    span: Span,
) -> Result<(), VmError> {
    let r = RegRef::from_name(name).ok_or_else(|| VmError::InvalidOperand {
        reason: format!("unknown register `{name}`"),
        span,
    })?;
    match (r, size) {
        (RegRef::R16(r), OpSize::Word) => {
            vm.cpu.set_r16(r, value);
            Ok(())
        }
        (RegRef::R8(r), OpSize::Byte) => {
            vm.cpu.set_r8(r, value as u8);
            Ok(())
        }
        (RegRef::R16(_), OpSize::Byte) => Err(VmError::InvalidOperand {
            reason: format!("byte write to 16-bit register `{name}`"),
            span,
        }),
        (RegRef::R8(_), OpSize::Word) => Err(VmError::InvalidOperand {
            reason: format!("word write to 8-bit register `{name}`"),
            span,
        }),
    }
}

fn read_memory(vm: &Vm, mem: &Mem, size: OpSize, span: Span) -> Result<u16, VmError> {
    let (seg, off) = effective_address(vm, mem, span)?;
    let phys = Memory::phys(seg, off);
    match size {
        OpSize::Byte => Ok(vm.mem.read_u8(phys)? as u16),
        OpSize::Word => Ok(vm.mem.read_u16(phys)?),
    }
}

fn write_memory(
    vm: &mut Vm,
    mem: &Mem,
    size: OpSize,
    value: u16,
    span: Span,
) -> Result<(), VmError> {
    let (seg, off) = effective_address(vm, mem, span)?;
    let phys = Memory::phys(seg, off);
    match size {
        OpSize::Byte => Ok(vm.mem.write_u8(phys, value as u8)?),
        OpSize::Word => Ok(vm.mem.write_u16(phys, value)?),
    }
}

fn reg16(name: &str, span: Span) -> Result<Option<Reg16>, VmError> {
    match RegRef::from_name(name) {
        Some(RegRef::R16(r)) => Ok(Some(r)),
        Some(RegRef::R8(_)) => Err(VmError::InvalidOperand {
            reason: format!("`{name}` is an 8-bit register"),
            span,
        }),
        None => Err(VmError::InvalidOperand {
            reason: format!("unknown register `{name}`"),
            span,
        }),
    }
}

fn default_segment(mem: &Mem) -> Reg16 {
    match mem.base.as_deref() {
        Some("bp") => Reg16::Ss,
        _ => Reg16::Ds,
    }
}
