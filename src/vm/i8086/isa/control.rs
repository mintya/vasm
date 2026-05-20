use crate::asm::ast::Operand;
use crate::asm::diagnostics::Span;
use crate::vm::i8086::exec::{Vm, VmError};
use crate::vm::i8086::isa::operand::{OpSize, read_operand};

pub fn loop_(vm: &mut Vm, ops: &[Operand], span: Span) -> Result<(), VmError> {
    if ops.len() != 1 {
        return Err(VmError::InvalidOperand {
            reason: format!("loop expects 1 operand, got {}", ops.len()),
            span,
        });
    }
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
