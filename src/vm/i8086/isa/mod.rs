pub mod arith;
pub mod control;
pub mod data_move;
pub mod flags;
pub mod operand;
pub mod stack;

use crate::asm::ast::Instruction;
use crate::asm::diagnostics::Span;
use crate::vm::i8086::exec::{Vm, VmError};

pub fn dispatch(vm: &mut Vm, instr: &Instruction, span: Span) -> Result<(), VmError> {
    match instr.mnemonic.as_str() {
        "mov" => data_move::mov(vm, &instr.operands, span),
        "xchg" => data_move::xchg(vm, &instr.operands, span),
        "push" => stack::push(vm, &instr.operands, span),
        "pop" => stack::pop(vm, &instr.operands, span),
        "pushf" => stack::pushf(vm, span),
        "popf" => stack::popf(vm, span),
        "add" => arith::add(vm, &instr.operands, span),
        "sub" => arith::sub(vm, &instr.operands, span),
        "inc" => arith::inc(vm, &instr.operands, span),
        "dec" => arith::dec(vm, &instr.operands, span),
        "loop" => control::loop_(vm, &instr.operands, span),
        "hlt" => control::hlt(vm),
        "nop" => control::nop(),
        m => Err(VmError::UnsupportedInstruction {
            mnemonic: m.into(),
            span,
        }),
    }
}
