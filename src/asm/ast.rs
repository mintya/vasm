use crate::asm::diagnostics::Span;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Program {
    pub segments: Vec<Segment>,
    pub entry: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub name: String,
    pub items: Vec<Item>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    Label(String, Span),
    DataDecl(DataDecl, Span),
    Assume(Vec<AssumeBinding>, Span),
    Instruction(Instruction, Span),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssumeBinding {
    pub seg_reg: String,
    pub segment: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataDecl {
    pub name: Option<String>,
    pub size: DataSize,
    pub values: Vec<DataValue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataSize {
    Byte,
    Word,
    Dword,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataValue {
    Expr(Expr),
    String(Vec<u8>),
    Dup { count: Expr, values: Vec<DataValue> },
    Uninit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    pub mnemonic: String,
    pub operands: Vec<Operand>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operand {
    Reg(String),
    Imm(Expr),
    Mem(Mem),
    Far { seg: u16, off: u16 },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Mem {
    pub seg_override: Option<String>,
    pub size: Option<DataSize>,
    pub base: Option<String>,
    pub index: Option<String>,
    pub disp: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Int(i64),
    Ident(String),
    Offset(Box<Expr>),
    Seg(Box<Expr>),
    Neg(Box<Expr>),
    BinOp {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}
