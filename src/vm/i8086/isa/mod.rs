pub mod arith;
pub mod control;
pub mod data_move;
pub mod doc;
pub mod flags;
pub mod intr;
pub mod io;
pub mod logic;
pub mod operand;
pub mod shift;
pub mod stack;

use crate::asm::ast::Instruction;
use crate::asm::diagnostics::Span;
use crate::vm::i8086::exec::{Vm, VmError};

pub fn dispatch(vm: &mut Vm, instr: &Instruction, span: Span) -> Result<(), VmError> {
    let m = instr.mnemonic.as_str();
    let ops = &instr.operands;
    match m {
        // data move
        "mov" => data_move::mov(vm, ops, span),
        "xchg" => data_move::xchg(vm, ops, span),
        // stack
        "push" => stack::push(vm, ops, span),
        "pop" => stack::pop(vm, ops, span),
        "pushf" => stack::pushf(vm, span),
        "popf" => stack::popf(vm, span),
        // arith
        "add" => arith::add(vm, ops, span),
        "sub" => arith::sub(vm, ops, span),
        "inc" => arith::inc(vm, ops, span),
        "dec" => arith::dec(vm, ops, span),
        "cmp" => arith::cmp(vm, ops, span),
        "neg" => arith::neg(vm, ops, span),
        "mul" => arith::mul(vm, ops, span),
        "div" => arith::div(vm, ops, span),
        // logic
        "and" => logic::and(vm, ops, span),
        "or" => logic::or(vm, ops, span),
        "xor" => logic::xor(vm, ops, span),
        "not" => logic::not(vm, ops, span),
        "test" => logic::test(vm, ops, span),
        // shift / rotate
        "shl" => shift::shl(vm, ops, span),
        "sal" => shift::sal(vm, ops, span),
        "shr" => shift::shr(vm, ops, span),
        "sar" => shift::sar(vm, ops, span),
        "rol" => shift::rol(vm, ops, span),
        "ror" => shift::ror(vm, ops, span),
        "rcl" => shift::rcl(vm, ops, span),
        "rcr" => shift::rcr(vm, ops, span),
        // control
        "loop" => control::loop_(vm, ops, span),
        "hlt" => control::hlt(vm),
        "nop" => control::nop(),
        "jmp" => control::jmp(vm, ops, span),
        "jcxz" => control::jcxz(vm, ops, span),
        "call" => control::call(vm, ops, span),
        "ret" => control::ret(vm, ops, span),
        "retf" => control::retf(vm, ops, span),
        // 条件跳转：18+ 个助记符走同一个 jcc 入口
        "je" | "jz" | "jne" | "jnz" | "js" | "jns" | "jo" | "jno" | "jp" | "jpe" | "jnp"
        | "jpo" | "jc" | "jb" | "jnae" | "jnc" | "jae" | "jnb" | "jbe" | "jna" | "ja" | "jnbe"
        | "jl" | "jnge" | "jge" | "jnl" | "jle" | "jng" | "jg" | "jnle" => {
            control::jcc(vm, m, ops, span)
        }
        // 中断与 I/O（M5）
        "int" => intr::int_(vm, ops, span),
        "iret" => intr::iret_(vm, ops, span),
        "cli" => intr::cli_(vm),
        "sti" => intr::sti_(vm),
        "in" => io::in_(vm, ops, span),
        "out" => io::out_(vm, ops, span),
        m => Err(VmError::UnsupportedInstruction {
            mnemonic: m.into(),
            span,
        }),
    }
}
