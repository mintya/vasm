use crate::asm::ast::{Expr, Operand};
use crate::asm::diagnostics::Span;
use crate::vm::i8086::cpu::Reg8;
use crate::vm::i8086::exec::{Vm, VmError};
use crate::vm::i8086::isa::flags;
use crate::vm::i8086::isa::operand::{OpSize, infer_size, read_operand, write_operand};

/// 8086 shift/rotate 共用入口。第 2 操作数支持 `1`、`cl`、立即数。
fn read_count(vm: &Vm, op: &Operand, span: Span) -> Result<u8, VmError> {
    match op {
        Operand::Imm(Expr::Int(n)) => Ok((*n as u32).min(255) as u8),
        Operand::Reg(name) if name.eq_ignore_ascii_case("cl") => Ok(vm.cpu.r8(Reg8::Cl)),
        Operand::Reg(name) => Err(VmError::InvalidOperand {
            reason: format!("shift count 只能是 1 / cl / 立即数，不能用 `{name}`"),
            span,
        }),
        _ => Err(VmError::InvalidOperand {
            reason: "shift count must be 1, cl, or immediate".into(),
            span,
        }),
    }
}

pub fn shl(vm: &mut Vm, ops: &[Operand], span: Span) -> Result<(), VmError> {
    expect_two(ops, "shl", span)?;
    let size = infer_size(ops).unwrap_or(OpSize::Word);
    let a = read_operand(vm, &ops[0], size, span)?;
    let count = read_count(vm, &ops[1], span)?;
    let result = do_shl(a, count, size, &mut vm.cpu.flags);
    write_operand(vm, &ops[0], size, result, span)
}

/// sal == shl
pub fn sal(vm: &mut Vm, ops: &[Operand], span: Span) -> Result<(), VmError> {
    shl(vm, ops, span)
}

pub fn shr(vm: &mut Vm, ops: &[Operand], span: Span) -> Result<(), VmError> {
    expect_two(ops, "shr", span)?;
    let size = infer_size(ops).unwrap_or(OpSize::Word);
    let a = read_operand(vm, &ops[0], size, span)?;
    let count = read_count(vm, &ops[1], span)?;
    let result = do_shr(a, count, size, &mut vm.cpu.flags);
    write_operand(vm, &ops[0], size, result, span)
}

pub fn sar(vm: &mut Vm, ops: &[Operand], span: Span) -> Result<(), VmError> {
    expect_two(ops, "sar", span)?;
    let size = infer_size(ops).unwrap_or(OpSize::Word);
    let a = read_operand(vm, &ops[0], size, span)?;
    let count = read_count(vm, &ops[1], span)?;
    let result = do_sar(a, count, size, &mut vm.cpu.flags);
    write_operand(vm, &ops[0], size, result, span)
}

pub fn rol(vm: &mut Vm, ops: &[Operand], span: Span) -> Result<(), VmError> {
    expect_two(ops, "rol", span)?;
    let size = infer_size(ops).unwrap_or(OpSize::Word);
    let a = read_operand(vm, &ops[0], size, span)?;
    let count = read_count(vm, &ops[1], span)?;
    let result = do_rol(a, count, size, &mut vm.cpu.flags);
    write_operand(vm, &ops[0], size, result, span)
}

pub fn ror(vm: &mut Vm, ops: &[Operand], span: Span) -> Result<(), VmError> {
    expect_two(ops, "ror", span)?;
    let size = infer_size(ops).unwrap_or(OpSize::Word);
    let a = read_operand(vm, &ops[0], size, span)?;
    let count = read_count(vm, &ops[1], span)?;
    let result = do_ror(a, count, size, &mut vm.cpu.flags);
    write_operand(vm, &ops[0], size, result, span)
}

pub fn rcl(vm: &mut Vm, ops: &[Operand], span: Span) -> Result<(), VmError> {
    expect_two(ops, "rcl", span)?;
    let size = infer_size(ops).unwrap_or(OpSize::Word);
    let a = read_operand(vm, &ops[0], size, span)?;
    let count = read_count(vm, &ops[1], span)?;
    let result = do_rcl(a, count, size, &mut vm.cpu.flags);
    write_operand(vm, &ops[0], size, result, span)
}

pub fn rcr(vm: &mut Vm, ops: &[Operand], span: Span) -> Result<(), VmError> {
    expect_two(ops, "rcr", span)?;
    let size = infer_size(ops).unwrap_or(OpSize::Word);
    let a = read_operand(vm, &ops[0], size, span)?;
    let count = read_count(vm, &ops[1], span)?;
    let result = do_rcr(a, count, size, &mut vm.cpu.flags);
    write_operand(vm, &ops[0], size, result, span)
}

// ----------------- 内核 -----------------

fn msb(value: u16, size: OpSize) -> bool {
    match size {
        OpSize::Byte => value & 0x80 != 0,
        OpSize::Word => value & 0x8000 != 0,
    }
}

fn mask(size: OpSize) -> u16 {
    match size {
        OpSize::Byte => 0x00FF,
        OpSize::Word => 0xFFFF,
    }
}

fn width(size: OpSize) -> u32 {
    match size {
        OpSize::Byte => 8,
        OpSize::Word => 16,
    }
}

fn set_szp(result: u16, size: OpSize, f: &mut crate::vm::i8086::cpu::Flags) {
    f.zf = (result & mask(size)) == 0;
    f.sf = msb(result, size);
    f.pf = flags::parity(result as u8);
}

fn do_shl(a: u16, count: u8, size: OpSize, f: &mut crate::vm::i8086::cpu::Flags) -> u16 {
    if count == 0 {
        return a & mask(size);
    }
    let w = width(size);
    let n = count as u32;
    let last_carry_bit = w.saturating_sub(n);
    let cf = if n <= w {
        ((a as u32) >> last_carry_bit) & 1 != 0
    } else {
        false
    };
    let result = if n >= w {
        0
    } else {
        ((a as u32) << n) as u16 & mask(size)
    };
    f.cf = cf;
    if count == 1 {
        f.of = msb(result, size) != cf;
    }
    set_szp(result, size, f);
    result
}

fn do_shr(a: u16, count: u8, size: OpSize, f: &mut crate::vm::i8086::cpu::Flags) -> u16 {
    if count == 0 {
        return a & mask(size);
    }
    let n = count as u32;
    let w = width(size);
    let val = (a & mask(size)) as u32;
    let cf = if n <= w {
        (val >> (n - 1)) & 1 != 0
    } else {
        false
    };
    let result = if n >= w { 0 } else { (val >> n) as u16 };
    f.cf = cf;
    if count == 1 {
        f.of = msb(a, size);
    }
    set_szp(result, size, f);
    result
}

fn do_sar(a: u16, count: u8, size: OpSize, f: &mut crate::vm::i8086::cpu::Flags) -> u16 {
    if count == 0 {
        return a & mask(size);
    }
    let n = count as u32;
    let result = match size {
        OpSize::Byte => {
            let s = (a as u8) as i8;
            let shift = n.min(7);
            (s >> shift) as u8 as u16
        }
        OpSize::Word => {
            let s = a as i16;
            let shift = n.min(15);
            (s >> shift) as u16
        }
    };
    // CF = 最后移出位；近似：算术移位移出的是符号位
    f.cf = msb(a, size);
    if count == 1 {
        f.of = false;
    }
    set_szp(result, size, f);
    result
}

fn do_rol(a: u16, count: u8, size: OpSize, f: &mut crate::vm::i8086::cpu::Flags) -> u16 {
    let w = width(size);
    let n = (count as u32) % w;
    let val = (a & mask(size)) as u32;
    let result = if n == 0 {
        val as u16
    } else {
        (((val << n) | (val >> (w - n))) as u16) & mask(size)
    };
    if count > 0 {
        f.cf = result & 1 != 0;
        if count == 1 {
            f.of = msb(result, size) != f.cf;
        }
    }
    result
}

fn do_ror(a: u16, count: u8, size: OpSize, f: &mut crate::vm::i8086::cpu::Flags) -> u16 {
    let w = width(size);
    let n = (count as u32) % w;
    let val = (a & mask(size)) as u32;
    let result = if n == 0 {
        val as u16
    } else {
        (((val >> n) | (val << (w - n))) as u16) & mask(size)
    };
    if count > 0 {
        f.cf = msb(result, size);
        if count == 1 {
            // OF = 顶两位异或
            let top = msb(result, size);
            let second = match size {
                OpSize::Byte => result & 0x40 != 0,
                OpSize::Word => result & 0x4000 != 0,
            };
            f.of = top != second;
        }
    }
    result
}

fn do_rcl(a: u16, count: u8, size: OpSize, f: &mut crate::vm::i8086::cpu::Flags) -> u16 {
    let w = width(size) + 1; // 通过 CF
    let mut n = (count as u32) % w;
    let mut val = (a & mask(size)) as u32;
    let mut cf = f.cf as u32;
    while n > 0 {
        let high = (val >> (width(size) - 1)) & 1;
        val = ((val << 1) | cf) & mask(size) as u32;
        cf = high;
        n -= 1;
    }
    f.cf = cf != 0;
    let result = val as u16;
    if count == 1 {
        f.of = msb(result, size) != f.cf;
    }
    result
}

fn do_rcr(a: u16, count: u8, size: OpSize, f: &mut crate::vm::i8086::cpu::Flags) -> u16 {
    let w = width(size) + 1;
    let mut n = (count as u32) % w;
    let mut val = (a & mask(size)) as u32;
    let mut cf = f.cf as u32;
    if count == 1 {
        let top_before = msb(a, size);
        f.of = top_before != (cf != 0);
    }
    while n > 0 {
        let low = val & 1;
        val = (val >> 1) | (cf << (width(size) - 1));
        cf = low;
        n -= 1;
    }
    f.cf = cf != 0;
    val as u16 & mask(size)
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
    fn shl_by_one_doubles_and_sets_cf_from_msb() {
        let mut vm = boot("code segment\n  mov ax, 8001h\n  shl ax, 1\n  hlt\ncode ends\nend\n");
        for _ in 0..3 {
            vm.step().unwrap();
        }
        assert_eq!(vm.cpu.ax, 0x0002);
        assert!(vm.cpu.flags.cf);
    }

    #[test]
    fn shr_lsb_to_cf() {
        let mut vm = boot("code segment\n  mov ax, 0003h\n  shr ax, 1\n  hlt\ncode ends\nend\n");
        for _ in 0..3 {
            vm.step().unwrap();
        }
        assert_eq!(vm.cpu.ax, 0x0001);
        assert!(vm.cpu.flags.cf);
    }

    #[test]
    fn sar_keeps_sign_bit() {
        let mut vm = boot("code segment\n  mov ax, 8000h\n  sar ax, 1\n  hlt\ncode ends\nend\n");
        for _ in 0..3 {
            vm.step().unwrap();
        }
        assert_eq!(vm.cpu.ax, 0xC000);
    }

    #[test]
    fn rol_wraps_msb_to_lsb() {
        let mut vm = boot("code segment\n  mov ax, 8001h\n  rol ax, 1\n  hlt\ncode ends\nend\n");
        for _ in 0..3 {
            vm.step().unwrap();
        }
        assert_eq!(vm.cpu.ax, 0x0003);
        assert!(vm.cpu.flags.cf); // CF = LSB after = 1
    }

    #[test]
    fn ror_wraps_lsb_to_msb() {
        let mut vm = boot("code segment\n  mov ax, 0001h\n  ror ax, 1\n  hlt\ncode ends\nend\n");
        for _ in 0..3 {
            vm.step().unwrap();
        }
        assert_eq!(vm.cpu.ax, 0x8000);
        assert!(vm.cpu.flags.cf);
    }

    #[test]
    fn shl_by_cl() {
        let mut vm =
            boot("code segment\n  mov ax, 1\n  mov cl, 4\n  shl ax, cl\n  hlt\ncode ends\nend\n");
        for _ in 0..4 {
            vm.step().unwrap();
        }
        assert_eq!(vm.cpu.ax, 0x0010);
    }
}
