use crate::asm::ast::Operand;
use crate::asm::diagnostics::Span;
use crate::vm::i8086::exec::{Vm, VmError};
use crate::vm::i8086::isa::operand::{
    OpSize, infer_size, is_seg_register, read_operand, write_operand,
};

pub fn mov(vm: &mut Vm, ops: &[Operand], span: Span) -> Result<(), VmError> {
    if ops.len() != 2 {
        return Err(VmError::InvalidOperand {
            reason: format!("mov expects 2 operands, got {}", ops.len()),
            span,
        });
    }
    if let (Operand::Reg(d), Operand::Imm(_)) = (&ops[0], &ops[1])
        && is_seg_register(d)
    {
        return Err(VmError::SegRegImmediate { span });
    }
    let size = infer_size(ops).unwrap_or(OpSize::Word);
    let value = read_operand(vm, &ops[1], size, span)?;
    write_operand(vm, &ops[0], size, value, span)
}

pub fn xchg(vm: &mut Vm, ops: &[Operand], span: Span) -> Result<(), VmError> {
    if ops.len() != 2 {
        return Err(VmError::InvalidOperand {
            reason: format!("xchg expects 2 operands, got {}", ops.len()),
            span,
        });
    }
    let size = infer_size(ops).unwrap_or(OpSize::Word);
    let a = read_operand(vm, &ops[0], size, span)?;
    let b = read_operand(vm, &ops[1], size, span)?;
    write_operand(vm, &ops[0], size, b, span)?;
    write_operand(vm, &ops[1], size, a, span)?;
    Ok(())
}
