use crate::asm::ast::Operand;
use crate::asm::diagnostics::Span;
use crate::vm::i8086::cpu::Flags;
use crate::vm::i8086::exec::{Vm, VmError};
use crate::vm::i8086::isa::operand::{OpSize, read_operand, write_operand};
use crate::vm::i8086::memory::Memory;

pub fn push(vm: &mut Vm, ops: &[Operand], span: Span) -> Result<(), VmError> {
    expect_one(ops, "push", span)?;
    let value = read_operand(vm, &ops[0], OpSize::Word, span)?;
    push_word(vm, value)
}

pub fn pop(vm: &mut Vm, ops: &[Operand], span: Span) -> Result<(), VmError> {
    expect_one(ops, "pop", span)?;
    let value = pop_word(vm)?;
    write_operand(vm, &ops[0], OpSize::Word, value, span)
}

pub fn pushf(vm: &mut Vm, _span: Span) -> Result<(), VmError> {
    let value = vm.cpu.flags.to_u16();
    push_word(vm, value)
}

pub fn popf(vm: &mut Vm, _span: Span) -> Result<(), VmError> {
    let value = pop_word(vm)?;
    vm.cpu.flags = Flags::from_u16(value);
    Ok(())
}

fn push_word(vm: &mut Vm, value: u16) -> Result<(), VmError> {
    let new_sp = vm.cpu.sp.wrapping_sub(2);
    let phys = Memory::phys(vm.cpu.ss, new_sp);
    vm.mem.write_u16(phys, value)?;
    vm.cpu.sp = new_sp;
    Ok(())
}

fn pop_word(vm: &mut Vm) -> Result<u16, VmError> {
    let phys = Memory::phys(vm.cpu.ss, vm.cpu.sp);
    let value = vm.mem.read_u16(phys)?;
    vm.cpu.sp = vm.cpu.sp.wrapping_add(2);
    Ok(value)
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
