use std::collections::HashMap;

use thiserror::Error;

use crate::asm::ast::{
    AssumeBinding, BinOp, DataDecl, DataSize, DataValue, Expr, Instruction, Item, Operand, Program,
};
use crate::asm::diagnostics::Span;
use crate::vm::i8086::memory::{MemError, Memory};

pub const DEFAULT_START_PARAGRAPH: u16 = 0x1000;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum LoadError {
    #[error("undefined symbol `{name}`")]
    UndefinedSymbol { name: String, span: Span },
    #[error("dup count must be a literal expression")]
    NonLiteralDupCount { span: Span },
    #[error("division by zero in constant expression")]
    DivByZero { span: Span },
    #[error("entry label `{name}` not found")]
    EntryNotFound { name: String },
    #[error("program size exceeds memory ({needed} bytes > {available})")]
    OutOfMemory { needed: u32, available: u32 },
    #[error("memory error: {0}")]
    Mem(#[from] MemError),
    #[error("`seg` operator requires a label or segment name")]
    InvalidSegOperator { span: Span },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SegmentKind {
    Code,
    Data,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    CodeLabel,
    DataByte,
    DataWord,
    DataDword,
    SegmentName,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolInfo {
    pub segment: String,
    pub offset: u16,
    pub kind: SymbolKind,
}

#[derive(Debug, Clone)]
pub struct InstrSlot {
    pub ip_offset: u16,
    pub size: u16,
    pub instr: Instruction,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct SegmentLayout {
    pub name: String,
    pub base_paragraph: u16,
    pub size_bytes: u32,
    pub kind: SegmentKind,
    pub instructions: Vec<InstrSlot>,
    pub assume: Vec<AssumeBinding>,
}

#[derive(Debug, Clone)]
pub struct LoadedProgram {
    pub segments: HashMap<String, SegmentLayout>,
    pub symbols: HashMap<String, SymbolInfo>,
    pub entry: Option<(String, u16)>,
}

pub fn load(
    program: &Program,
    mem_kb: u32,
    start_paragraph: u16,
) -> Result<(LoadedProgram, Memory), LoadError> {
    let mut layouts: HashMap<String, SegmentLayout> = HashMap::new();
    let mut symbols: HashMap<String, SymbolInfo> = HashMap::new();
    let mut next_paragraph = start_paragraph;

    // Pass 1: 段布局 + 符号表（不解析需要符号的常量）
    for seg in &program.segments {
        let kind = if seg
            .items
            .iter()
            .any(|it| matches!(it, Item::Instruction(_, _)))
        {
            SegmentKind::Code
        } else {
            SegmentKind::Data
        };

        let base = next_paragraph;
        let mut offset: u32 = 0;
        let mut instructions = Vec::new();
        let mut assume = Vec::new();

        for item in &seg.items {
            match item {
                Item::Label(name, _span) => {
                    symbols.insert(
                        name.clone(),
                        SymbolInfo {
                            segment: seg.name.clone(),
                            offset: offset as u16,
                            kind: SymbolKind::CodeLabel,
                        },
                    );
                }
                Item::DataDecl(decl, _span) => {
                    if let Some(name) = &decl.name {
                        symbols.insert(
                            name.clone(),
                            SymbolInfo {
                                segment: seg.name.clone(),
                                offset: offset as u16,
                                kind: kind_from_size(decl.size),
                            },
                        );
                    }
                    let bytes = decl_byte_size(decl)?;
                    offset += bytes;
                }
                Item::Instruction(instr, span) => {
                    let size = instr_size_estimate(instr);
                    instructions.push(InstrSlot {
                        ip_offset: offset as u16,
                        size,
                        instr: instr.clone(),
                        span: *span,
                    });
                    offset += size as u32;
                }
                Item::Assume(bindings, _span) => {
                    assume.extend_from_slice(bindings);
                }
            }
        }

        // 段名本身也是符号
        symbols.insert(
            seg.name.clone(),
            SymbolInfo {
                segment: seg.name.clone(),
                offset: 0,
                kind: SymbolKind::SegmentName,
            },
        );

        let size_bytes = offset;
        let para_size = size_bytes.div_ceil(16);
        next_paragraph = next_paragraph.saturating_add(para_size as u16);

        layouts.insert(
            seg.name.clone(),
            SegmentLayout {
                name: seg.name.clone(),
                base_paragraph: base,
                size_bytes,
                kind,
                instructions,
                assume,
            },
        );
    }

    // 入口
    let entry = match &program.entry {
        Some(label) => {
            let sym = symbols.get(label).ok_or_else(|| LoadError::EntryNotFound {
                name: label.clone(),
            })?;
            Some((sym.segment.clone(), sym.offset))
        }
        None => program.segments.iter().find_map(|seg| {
            let layout = layouts.get(&seg.name)?;
            if layout.kind != SegmentKind::Code {
                return None;
            }
            layout
                .instructions
                .first()
                .map(|s| (seg.name.clone(), s.ip_offset))
        }),
    };

    // 准备内存 + Pass 2：把数据段的实际字节写进去
    let mut memory = Memory::new(mem_kb);
    for seg in &program.segments {
        let layout = &layouts[&seg.name];
        if layout.kind != SegmentKind::Data {
            continue;
        }
        let mut buf: Vec<u8> = Vec::with_capacity(layout.size_bytes as usize);
        for item in &seg.items {
            if let Item::DataDecl(decl, _span) = item {
                encode_decl(decl, &symbols, &layouts, &mut buf)?;
            }
        }
        let phys = Memory::phys(layout.base_paragraph, 0);
        if phys as u64 + buf.len() as u64 > memory.size() as u64 {
            return Err(LoadError::OutOfMemory {
                needed: phys + buf.len() as u32,
                available: memory.size(),
            });
        }
        memory.write_bytes(phys, &buf)?;
    }

    let loaded = LoadedProgram {
        segments: layouts,
        symbols,
        entry,
    };
    Ok((loaded, memory))
}

fn kind_from_size(s: DataSize) -> SymbolKind {
    match s {
        DataSize::Byte => SymbolKind::DataByte,
        DataSize::Word => SymbolKind::DataWord,
        DataSize::Dword => SymbolKind::DataDword,
    }
}

fn elem_bytes(s: DataSize) -> u32 {
    match s {
        DataSize::Byte => 1,
        DataSize::Word => 2,
        DataSize::Dword => 4,
    }
}

fn decl_byte_size(decl: &DataDecl) -> Result<u32, LoadError> {
    let elem = elem_bytes(decl.size);
    let mut total = 0u32;
    for v in &decl.values {
        total += value_byte_size(v, elem)?;
    }
    Ok(total)
}

fn value_byte_size(v: &DataValue, elem: u32) -> Result<u32, LoadError> {
    match v {
        DataValue::Expr(_) | DataValue::Uninit => Ok(elem),
        DataValue::String(bytes) => Ok(bytes.len() as u32),
        DataValue::Dup { count, values } => {
            let n = const_eval_literal(count, Span::new(0, 0, 0))?;
            let inner: u32 = values
                .iter()
                .map(|v| value_byte_size(v, elem))
                .sum::<Result<u32, _>>()?;
            Ok(n.unsigned_abs() as u32 * inner)
        }
    }
}

fn const_eval_literal(expr: &Expr, span: Span) -> Result<i64, LoadError> {
    match expr {
        Expr::Int(n) => Ok(*n),
        Expr::Neg(inner) => Ok(-const_eval_literal(inner, span)?),
        Expr::BinOp { op, lhs, rhs } => {
            let l = const_eval_literal(lhs, span)?;
            let r = const_eval_literal(rhs, span)?;
            apply_binop(*op, l, r, span)
        }
        Expr::Ident(_) | Expr::Offset(_) | Expr::Seg(_) => {
            Err(LoadError::NonLiteralDupCount { span })
        }
    }
}

fn apply_binop(op: BinOp, l: i64, r: i64, span: Span) -> Result<i64, LoadError> {
    Ok(match op {
        BinOp::Add => l.wrapping_add(r),
        BinOp::Sub => l.wrapping_sub(r),
        BinOp::Mul => l.wrapping_mul(r),
        BinOp::Div => {
            if r == 0 {
                return Err(LoadError::DivByZero { span });
            }
            l.wrapping_div(r)
        }
        BinOp::Mod => {
            if r == 0 {
                return Err(LoadError::DivByZero { span });
            }
            l.wrapping_rem(r)
        }
    })
}

fn eval_full(
    expr: &Expr,
    symbols: &HashMap<String, SymbolInfo>,
    layouts: &HashMap<String, SegmentLayout>,
    span: Span,
) -> Result<i64, LoadError> {
    match expr {
        Expr::Int(n) => Ok(*n),
        Expr::Neg(inner) => Ok(-eval_full(inner, symbols, layouts, span)?),
        Expr::BinOp { op, lhs, rhs } => {
            let l = eval_full(lhs, symbols, layouts, span)?;
            let r = eval_full(rhs, symbols, layouts, span)?;
            apply_binop(*op, l, r, span)
        }
        Expr::Ident(name) => resolve_ident(name, symbols, layouts, span),
        Expr::Offset(inner) => {
            // offset(label) = 标签在段内的偏移；与裸 Ident 等同
            match inner.as_ref() {
                Expr::Ident(name) => resolve_ident(name, symbols, layouts, span),
                _ => eval_full(inner, symbols, layouts, span),
            }
        }
        Expr::Seg(inner) => match inner.as_ref() {
            Expr::Ident(name) => {
                let sym = symbols
                    .get(name)
                    .ok_or_else(|| LoadError::UndefinedSymbol {
                        name: name.clone(),
                        span,
                    })?;
                let seg = &layouts[&sym.segment];
                Ok(seg.base_paragraph as i64)
            }
            _ => Err(LoadError::InvalidSegOperator { span }),
        },
    }
}

fn resolve_ident(
    name: &str,
    symbols: &HashMap<String, SymbolInfo>,
    layouts: &HashMap<String, SegmentLayout>,
    span: Span,
) -> Result<i64, LoadError> {
    let sym = symbols
        .get(name)
        .ok_or_else(|| LoadError::UndefinedSymbol {
            name: name.to_string(),
            span,
        })?;
    match sym.kind {
        SymbolKind::SegmentName => Ok(layouts[&sym.segment].base_paragraph as i64),
        _ => Ok(sym.offset as i64),
    }
}

fn encode_decl(
    decl: &DataDecl,
    symbols: &HashMap<String, SymbolInfo>,
    layouts: &HashMap<String, SegmentLayout>,
    out: &mut Vec<u8>,
) -> Result<(), LoadError> {
    let elem = elem_bytes(decl.size);
    for v in &decl.values {
        encode_value(v, elem, symbols, layouts, out)?;
    }
    Ok(())
}

fn encode_value(
    v: &DataValue,
    elem: u32,
    symbols: &HashMap<String, SymbolInfo>,
    layouts: &HashMap<String, SegmentLayout>,
    out: &mut Vec<u8>,
) -> Result<(), LoadError> {
    match v {
        DataValue::Expr(e) => {
            let value = eval_full(e, symbols, layouts, Span::new(0, 0, 0))?;
            push_le(out, value, elem);
        }
        DataValue::String(bytes) => out.extend_from_slice(bytes),
        DataValue::Uninit => {
            for _ in 0..elem {
                out.push(0);
            }
        }
        DataValue::Dup { count, values } => {
            let n = const_eval_literal(count, Span::new(0, 0, 0))?;
            let mut inner: Vec<u8> = Vec::new();
            for v in values {
                encode_value(v, elem, symbols, layouts, &mut inner)?;
            }
            for _ in 0..n.unsigned_abs() as u32 {
                out.extend_from_slice(&inner);
            }
        }
    }
    Ok(())
}

fn push_le(out: &mut Vec<u8>, value: i64, elem_bytes: u32) {
    let bytes = value.to_le_bytes();
    out.extend_from_slice(&bytes[..elem_bytes as usize]);
}

/// 粗估每条指令的字节大小，仅用于给 ip 一个合理递增量。
pub fn instr_size_estimate(instr: &Instruction) -> u16 {
    let m = instr.mnemonic.as_str();
    let ops = &instr.operands;
    match m {
        "hlt" | "nop" | "pushf" | "popf" => 1,
        "push" | "pop" | "inc" | "dec" if single_reg16(ops) => 1,
        "ret" | "retf" if ops.is_empty() => 1,
        "ret" | "retf" => 3, // ret imm16 / retf imm16
        "loop" | "jcxz" => 2,
        "jmp" => match ops.first() {
            Some(Operand::Far { .. }) => 5,
            _ => 3,
        },
        "call" => match ops.first() {
            Some(Operand::Far { .. }) => 5,
            _ => 3,
        },
        "mov" => mov_size(ops),
        "add" | "sub" | "and" | "or" | "xor" | "cmp" | "test" => arith_size(ops),
        "mul" | "div" | "neg" | "not" => 2,
        "shl" | "sal" | "shr" | "sar" | "rol" | "ror" | "rcl" | "rcr" => shift_size(ops),
        "xchg" => xchg_size(ops),
        // 所有 jcc：短跳 2 字节
        "je" | "jz" | "jne" | "jnz" | "js" | "jns" | "jo" | "jno" | "jp" | "jpe" | "jnp"
        | "jpo" | "jc" | "jb" | "jnae" | "jnc" | "jae" | "jnb" | "jbe" | "jna" | "ja" | "jnbe"
        | "jl" | "jnge" | "jge" | "jnl" | "jle" | "jng" | "jg" | "jnle" => 2,
        _ => 3,
    }
}

fn shift_size(ops: &[Operand]) -> u16 {
    // count 是 1 或 cl 都按 2 字节；其他立即数按 3 字节（80186+）。
    match ops {
        [_, Operand::Imm(crate::asm::ast::Expr::Int(1))] => 2,
        [_, Operand::Reg(name)] if name.eq_ignore_ascii_case("cl") => 2,
        [_, Operand::Imm(_)] => 3,
        _ => 2,
    }
}

fn single_reg16(ops: &[Operand]) -> bool {
    matches!(ops, [Operand::Reg(name)] if is_16bit_reg(name))
}

fn is_16bit_reg(name: &str) -> bool {
    matches!(
        name,
        "ax" | "bx" | "cx" | "dx" | "si" | "di" | "bp" | "sp" | "cs" | "ds" | "ss" | "es"
    )
}

fn is_8bit_reg(name: &str) -> bool {
    matches!(name, "al" | "ah" | "bl" | "bh" | "cl" | "ch" | "dl" | "dh")
}

fn mov_size(ops: &[Operand]) -> u16 {
    match ops {
        [Operand::Reg(_), Operand::Reg(_)] => 2,
        [Operand::Reg(d), Operand::Imm(_)] if is_8bit_reg(d) => 2,
        [Operand::Reg(_), Operand::Imm(_)] => 3,
        [Operand::Reg(_), Operand::Mem(_)] | [Operand::Mem(_), Operand::Reg(_)] => 3,
        [Operand::Mem(_), Operand::Imm(_)] => 4,
        _ => 3,
    }
}

fn arith_size(ops: &[Operand]) -> u16 {
    match ops {
        [Operand::Reg(_), Operand::Reg(_)] => 2,
        [Operand::Reg(d), Operand::Imm(_)] => {
            if is_8bit_reg(d) {
                3
            } else {
                4
            }
        }
        [Operand::Reg(_), Operand::Mem(_)] | [Operand::Mem(_), Operand::Reg(_)] => 3,
        _ => 3,
    }
}

fn xchg_size(ops: &[Operand]) -> u16 {
    match ops {
        // xchg ax, reg16 是 1 字节短形式
        [Operand::Reg(a), Operand::Reg(b)] if (a == "ax" || b == "ax") && a != b => 1,
        [Operand::Reg(_), Operand::Reg(_)] => 2,
        _ => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asm::parser::parse;

    fn load_str(src: &str) -> Result<(LoadedProgram, Memory), LoadError> {
        let (prog, diags) = parse(src);
        assert!(diags.is_empty(), "parse diags: {diags:?}");
        load(&prog, 1024, DEFAULT_START_PARAGRAPH)
    }

    #[test]
    fn lays_out_single_code_segment() {
        let src = "code segment\n  mov ax, 1\n  mov bx, 2\n  hlt\ncode ends\nend\n";
        let (loaded, _mem) = load_str(src).unwrap();
        let code = &loaded.segments["code"];
        assert_eq!(code.kind, SegmentKind::Code);
        assert_eq!(code.base_paragraph, DEFAULT_START_PARAGRAPH);
        assert_eq!(code.instructions.len(), 3);
        // mov reg16, imm16 = 3, mov reg16, imm16 = 3, hlt = 1 → offsets 0, 3, 6, total 7
        assert_eq!(code.instructions[0].ip_offset, 0);
        assert_eq!(code.instructions[1].ip_offset, 3);
        assert_eq!(code.instructions[2].ip_offset, 6);
    }

    #[test]
    fn assigns_paragraphs_to_multiple_segments() {
        let src = "data segment\n  msg db 'hi'\ndata ends\ncode segment\n  hlt\ncode ends\nend\n";
        let (loaded, _mem) = load_str(src).unwrap();
        let data = &loaded.segments["data"];
        let code = &loaded.segments["code"];
        assert_eq!(data.base_paragraph, DEFAULT_START_PARAGRAPH);
        // data 段 2 字节，向上对齐到 16 → code 段下一个 paragraph
        assert_eq!(code.base_paragraph, DEFAULT_START_PARAGRAPH + 1);
    }

    #[test]
    fn data_label_symbol_kind() {
        let src = "data segment\n  msg db 'hello'\n  cnt dw 1\n  big dd 2\ndata ends\nend\n";
        let (loaded, _) = load_str(src).unwrap();
        assert_eq!(loaded.symbols["msg"].kind, SymbolKind::DataByte);
        assert_eq!(loaded.symbols["cnt"].kind, SymbolKind::DataWord);
        assert_eq!(loaded.symbols["big"].kind, SymbolKind::DataDword);
        // 5 bytes hello + 0 padding = msg at 0; cnt at 5; big at 7
        assert_eq!(loaded.symbols["msg"].offset, 0);
        assert_eq!(loaded.symbols["cnt"].offset, 5);
        assert_eq!(loaded.symbols["big"].offset, 7);
    }

    #[test]
    fn segment_name_resolves_to_paragraph() {
        let src = "data segment\n  db 0\ndata ends\nend\n";
        let (loaded, _) = load_str(src).unwrap();
        assert_eq!(loaded.symbols["data"].kind, SymbolKind::SegmentName);
    }

    #[test]
    fn entry_label_picked_up_from_end_directive() {
        let src = "code segment\nstart:\n  hlt\ncode ends\nend start\n";
        let (loaded, _) = load_str(src).unwrap();
        assert_eq!(loaded.entry, Some(("code".into(), 0)));
    }

    #[test]
    fn entry_defaults_to_first_code_instruction() {
        let src = "code segment\n  hlt\ncode ends\nend\n";
        let (loaded, _) = load_str(src).unwrap();
        assert_eq!(loaded.entry, Some(("code".into(), 0)));
    }

    #[test]
    fn data_segment_bytes_written_to_memory() {
        let src = "data segment\n  msg db 'AB'\ndata ends\nend\n";
        let (loaded, mem) = load_str(src).unwrap();
        let base = loaded.segments["data"].base_paragraph;
        let phys = Memory::phys(base, 0);
        assert_eq!(mem.read_u8(phys).unwrap(), b'A');
        assert_eq!(mem.read_u8(phys + 1).unwrap(), b'B');
    }

    #[test]
    fn dup_expands_values() {
        let src = "data segment\n  arr db 3 dup (0aah)\ndata ends\nend\n";
        let (loaded, mem) = load_str(src).unwrap();
        let phys = Memory::phys(loaded.segments["data"].base_paragraph, 0);
        for i in 0..3 {
            assert_eq!(mem.read_u8(phys + i).unwrap(), 0xAA);
        }
    }

    #[test]
    fn word_data_is_little_endian() {
        let src = "data segment\n  w dw 1234h\ndata ends\nend\n";
        let (loaded, mem) = load_str(src).unwrap();
        let phys = Memory::phys(loaded.segments["data"].base_paragraph, 0);
        assert_eq!(mem.read_u8(phys).unwrap(), 0x34);
        assert_eq!(mem.read_u8(phys + 1).unwrap(), 0x12);
    }

    #[test]
    fn missing_entry_label_errors() {
        let src = "code segment\n  hlt\ncode ends\nend nowhere\n";
        let err = load_str(src).unwrap_err();
        assert!(matches!(err, LoadError::EntryNotFound { .. }));
    }

    #[test]
    fn data_only_program_loads_with_no_entry() {
        let src = "data segment\n  db 0\ndata ends\n";
        let (loaded, _) = load_str(src).unwrap();
        assert!(loaded.entry.is_none());
    }
}
