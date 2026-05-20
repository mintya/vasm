//! `in` / `out` 端口 I/O。
//!
//! 教学只实装一个端口：`60h` 键盘扫描码寄存器（教材 §14 用到）。
//!
//! - `in al, 60h` / `in al, dx` (dx=60h) → 从 console 输入缓冲弹一字节；空则返 0
//! - `out 60h, al` → 写键盘控制端口（忽略，教学不需要副作用）
//!
//! 其他端口一律返 `UnsupportedPort`，让用户在 TUI 中看到明确错误。

use crate::asm::ast::Operand;
use crate::asm::diagnostics::Span;
use crate::vm::i8086::exec::{Vm, VmError};
use crate::vm::i8086::isa::operand::{OpSize, infer_size, read_operand, write_operand};

const KEYBOARD_DATA_PORT: u16 = 0x60;

pub fn in_(vm: &mut Vm, ops: &[Operand], span: Span) -> Result<(), VmError> {
    if ops.len() != 2 {
        return Err(VmError::InvalidOperand {
            reason: format!("in expects 2 operands, got {}", ops.len()),
            span,
        });
    }
    let port = read_port(vm, &ops[1], span)?;
    let size = infer_size(&ops[..1]).unwrap_or(OpSize::Byte);
    let value = match port {
        KEYBOARD_DATA_PORT => vm.console.pop_input().unwrap_or(0) as u16,
        _ => return Err(VmError::UnsupportedPort { port, span }),
    };
    write_operand(vm, &ops[0], size, value, span)
}

pub fn out_(vm: &mut Vm, ops: &[Operand], span: Span) -> Result<(), VmError> {
    if ops.len() != 2 {
        return Err(VmError::InvalidOperand {
            reason: format!("out expects 2 operands, got {}", ops.len()),
            span,
        });
    }
    let port = read_port(vm, &ops[0], span)?;
    let size = infer_size(&ops[1..]).unwrap_or(OpSize::Byte);
    let _value = read_operand(vm, &ops[1], size, span)?;
    match port {
        KEYBOARD_DATA_PORT => Ok(()), // 写忽略
        _ => Err(VmError::UnsupportedPort { port, span }),
    }
}

/// 端口操作数：立即数（imm8/imm16）或 `dx` 寄存器。
fn read_port(vm: &Vm, op: &Operand, span: Span) -> Result<u16, VmError> {
    match op {
        Operand::Reg(name) if name == "dx" => Ok(vm.cpu.dx),
        Operand::Reg(name) => Err(VmError::InvalidOperand {
            reason: format!("in/out port must be imm or dx, got `{name}`"),
            span,
        }),
        Operand::Imm(_) => Ok(read_operand(vm, op, OpSize::Word, span)?),
        Operand::Mem(_) | Operand::Far { .. } => Err(VmError::InvalidOperand {
            reason: "in/out port must be imm or dx".into(),
            span,
        }),
    }
}
