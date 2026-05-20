use crate::asm::ast::Operand;
use crate::asm::diagnostics::Span;
use crate::vm::i8086::exec::{Vm, VmError};
use crate::vm::i8086::isa::flags;
use crate::vm::i8086::isa::operand::{OpSize, infer_size, read_operand, write_operand};

pub fn and(vm: &mut Vm, ops: &[Operand], span: Span) -> Result<(), VmError> {
    bitwise(vm, ops, span, "and", |a, b| a & b, |a, b| a & b)
}

pub fn or(vm: &mut Vm, ops: &[Operand], span: Span) -> Result<(), VmError> {
    bitwise(vm, ops, span, "or", |a, b| a | b, |a, b| a | b)
}

pub fn xor(vm: &mut Vm, ops: &[Operand], span: Span) -> Result<(), VmError> {
    bitwise(vm, ops, span, "xor", |a, b| a ^ b, |a, b| a ^ b)
}

/// test = and 但不写回。
pub fn test(vm: &mut Vm, ops: &[Operand], span: Span) -> Result<(), VmError> {
    expect_two(ops, "test", span)?;
    let size = infer_size(ops).unwrap_or(OpSize::Word);
    let a = read_operand(vm, &ops[0], size, span)?;
    let b = read_operand(vm, &ops[1], size, span)?;
    match size {
        OpSize::Byte => flags::after_logical_u8((a as u8) & (b as u8), &mut vm.cpu.flags),
        OpSize::Word => flags::after_logical_u16(a & b, &mut vm.cpu.flags),
    }
    Ok(())
}

/// not 不动任何 flag。
pub fn not(vm: &mut Vm, ops: &[Operand], span: Span) -> Result<(), VmError> {
    expect_one(ops, "not", span)?;
    let size = infer_size(ops).unwrap_or(OpSize::Word);
    let a = read_operand(vm, &ops[0], size, span)?;
    let result = match size {
        OpSize::Byte => (!(a as u8)) as u16,
        OpSize::Word => !a,
    };
    write_operand(vm, &ops[0], size, result, span)
}

fn bitwise(
    vm: &mut Vm,
    ops: &[Operand],
    span: Span,
    name: &str,
    op8: fn(u8, u8) -> u8,
    op16: fn(u16, u16) -> u16,
) -> Result<(), VmError> {
    expect_two(ops, name, span)?;
    let size = infer_size(ops).unwrap_or(OpSize::Word);
    let a = read_operand(vm, &ops[0], size, span)?;
    let b = read_operand(vm, &ops[1], size, span)?;
    let result = match size {
        OpSize::Byte => {
            let r = op8(a as u8, b as u8);
            flags::after_logical_u8(r, &mut vm.cpu.flags);
            r as u16
        }
        OpSize::Word => {
            let r = op16(a, b);
            flags::after_logical_u16(r, &mut vm.cpu.flags);
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

#[cfg(test)]
mod tests {
    use crate::asm::parser;
    use crate::vm::i8086::exec::Vm;

    fn boot(src: &str) -> Vm {
        let (p, d) = parser::parse(src);
        assert!(d.is_empty(), "{d:?}");
        Vm::boot(p, 1024).expect("boot")
    }

    #[test]
    fn and_zeroes_cf_of_and_sets_zf() {
        let mut vm = boot("code segment\n  mov ax, 0F0Fh\n  and ax, 0F0h\n  hlt\ncode ends\nend\n");
        for _ in 0..3 {
            vm.step().unwrap();
        }
        assert_eq!(vm.cpu.ax, 0x000F & 0x00F0); // 0
        assert!(vm.cpu.flags.zf);
        assert!(!vm.cpu.flags.cf);
        assert!(!vm.cpu.flags.of);
    }

    #[test]
    fn xor_zeros_register() {
        let mut vm = boot("code segment\n  mov ax, 0FFFFh\n  xor ax, ax\n  hlt\ncode ends\nend\n");
        for _ in 0..3 {
            vm.step().unwrap();
        }
        assert_eq!(vm.cpu.ax, 0);
        assert!(vm.cpu.flags.zf);
    }

    #[test]
    fn or_sets_sf_when_high_bit() {
        let mut vm = boot("code segment\n  mov ax, 8000h\n  or ax, 1\n  hlt\ncode ends\nend\n");
        for _ in 0..3 {
            vm.step().unwrap();
        }
        assert_eq!(vm.cpu.ax, 0x8001);
        assert!(vm.cpu.flags.sf);
    }

    #[test]
    fn not_preserves_flags() {
        let mut vm = boot("code segment\n  mov ax, 0FFFFh\n  not ax\n  hlt\ncode ends\nend\n");
        for _ in 0..3 {
            vm.step().unwrap();
        }
        assert_eq!(vm.cpu.ax, 0);
        assert!(!vm.cpu.flags.zf);
    }

    #[test]
    fn test_does_not_write() {
        let mut vm = boot("code segment\n  mov ax, 0Fh\n  test ax, 0F0h\n  hlt\ncode ends\nend\n");
        for _ in 0..3 {
            vm.step().unwrap();
        }
        assert_eq!(vm.cpu.ax, 0x000F);
        assert!(vm.cpu.flags.zf);
    }
}
