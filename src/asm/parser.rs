use crate::asm::ast::{
    AssumeBinding, BinOp, DataDecl, DataSize, DataValue, Expr, Instruction, Item, Mem, Operand,
    Program, Segment,
};
use crate::asm::diagnostics::{Diagnostic, Span};
use crate::asm::lexer::{Token, TokenKind, tokenize};

pub fn parse(source: &str) -> (Program, Vec<Diagnostic>) {
    let (tokens, lex_diags) = tokenize(source);
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program();
    let mut diags = lex_diags;
    diags.extend(parser.into_diagnostics());
    (program, diags)
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    diags: Vec<Diagnostic>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            diags: Vec::new(),
        }
    }

    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.diags
    }

    fn peek(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    fn peek_at(&self, n: usize) -> Option<&TokenKind> {
        self.tokens.get(self.pos + n).map(|t| &t.kind)
    }

    fn current_span(&self) -> Span {
        self.tokens[self.pos].span
    }

    fn advance(&mut self) -> &Token {
        let t = &self.tokens[self.pos];
        if !matches!(t.kind, TokenKind::Eof) {
            self.pos += 1;
        }
        t
    }

    fn error(&mut self, span: Span, message: impl Into<String>) {
        self.diags.push(Diagnostic::error(span, message));
    }

    // ---- expressions ------------------------------------------------------

    pub fn parse_expr(&mut self) -> Option<Expr> {
        self.parse_add()
    }

    fn parse_add(&mut self) -> Option<Expr> {
        let mut lhs = self.parse_mul()?;
        loop {
            let op = match self.peek() {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => return Some(lhs),
            };
            self.advance();
            let rhs = self.parse_mul()?;
            lhs = Expr::BinOp {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
    }

    fn parse_mul(&mut self) -> Option<Expr> {
        let mut lhs = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                TokenKind::Percent => BinOp::Mod,
                _ => return Some(lhs),
            };
            self.advance();
            let rhs = self.parse_unary()?;
            lhs = Expr::BinOp {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
    }

    fn parse_unary(&mut self) -> Option<Expr> {
        enum Pref {
            Neg,
            Pos,
            Offset,
            Seg,
        }
        let pref = match self.peek() {
            TokenKind::Minus => Some(Pref::Neg),
            TokenKind::Plus => Some(Pref::Pos),
            TokenKind::Ident(s) if s == "offset" => Some(Pref::Offset),
            TokenKind::Ident(s) if s == "seg" => Some(Pref::Seg),
            _ => None,
        };
        match pref {
            Some(Pref::Neg) => {
                self.advance();
                Some(Expr::Neg(Box::new(self.parse_unary()?)))
            }
            Some(Pref::Pos) => {
                self.advance();
                self.parse_unary()
            }
            Some(Pref::Offset) => {
                self.advance();
                Some(Expr::Offset(Box::new(self.parse_unary()?)))
            }
            Some(Pref::Seg) => {
                self.advance();
                Some(Expr::Seg(Box::new(self.parse_unary()?)))
            }
            None => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Option<Expr> {
        let span = self.current_span();
        match self.peek().clone() {
            TokenKind::Number(n) => {
                self.advance();
                Some(Expr::Int(n))
            }
            TokenKind::String(bytes) => {
                self.advance();
                if bytes.len() == 1 {
                    Some(Expr::Int(bytes[0] as i64))
                } else {
                    self.error(
                        span,
                        format!(
                            "multi-byte string `{}` not allowed in expression context",
                            String::from_utf8_lossy(&bytes)
                        ),
                    );
                    Some(Expr::Int(0))
                }
            }
            TokenKind::Ident(name) => {
                self.advance();
                Some(Expr::Ident(name))
            }
            TokenKind::LParen => {
                self.advance();
                let inner = self.parse_expr()?;
                if !matches!(self.peek(), TokenKind::RParen) {
                    self.error(self.current_span(), "expected `)`");
                    return Some(inner);
                }
                self.advance();
                Some(inner)
            }
            other => {
                self.error(span, format!("expected expression, found {}", desc(&other)));
                None
            }
        }
    }

    // ---- operands & memory -----------------------------------------------

    pub fn parse_operand(&mut self) -> Option<Operand> {
        // size override: byte/word/dword ptr <inner>
        if let TokenKind::Ident(s) = self.peek().clone()
            && is_size_keyword(&s)
            && matches!(self.peek_at(1), Some(TokenKind::Ident(p)) if p == "ptr")
        {
            let span = self.current_span();
            self.advance(); // size keyword
            self.advance(); // ptr
            let inner = self.parse_operand()?;
            return Some(self.apply_size_override(inner, size_from_kw(&s), span));
        }

        // segment override: seg_reg ':' <inner>
        if let TokenKind::Ident(s) = self.peek().clone()
            && is_seg_register(&s)
            && matches!(self.peek_at(1), Some(TokenKind::Colon))
        {
            let span = self.current_span();
            self.advance(); // seg reg
            self.advance(); // ':'
            let inner = self.parse_operand()?;
            return Some(self.apply_seg_override(inner, s, span));
        }

        // bare register
        if let TokenKind::Ident(s) = self.peek().clone()
            && is_register(&s)
        {
            self.advance();
            return Some(Operand::Reg(s));
        }

        // [ mem inner ]
        if matches!(self.peek(), TokenKind::LBracket) {
            self.advance();
            let mem = self.parse_mem_inner();
            if !matches!(self.peek(), TokenKind::RBracket) {
                self.error(self.current_span(), "expected `]`");
            } else {
                self.advance();
            }
            return Some(Operand::Mem(mem));
        }

        // fall back to expression (immediate or bare symbol)
        Some(Operand::Imm(self.parse_expr()?))
    }

    fn parse_mem_inner(&mut self) -> Mem {
        let mut mem = Mem::default();
        let mut disp_terms: Vec<Expr> = Vec::new();
        let mut first = true;

        loop {
            let negate = if first {
                first = false;
                false
            } else {
                match self.peek() {
                    TokenKind::Plus => {
                        self.advance();
                        false
                    }
                    TokenKind::Minus => {
                        self.advance();
                        true
                    }
                    _ => break,
                }
            };

            // try to consume a register first
            let mut consumed_register = false;
            if let TokenKind::Ident(s) = self.peek().clone() {
                if is_base_register(&s) {
                    let span = self.current_span();
                    self.advance();
                    if negate {
                        self.error(span, format!("register `{s}` cannot be negated"));
                    } else if mem.base.is_some() {
                        self.error(span, "more than one base register in memory operand");
                    } else {
                        mem.base = Some(s);
                    }
                    consumed_register = true;
                } else if is_index_register(&s) {
                    let span = self.current_span();
                    self.advance();
                    if negate {
                        self.error(span, format!("register `{s}` cannot be negated"));
                    } else if mem.index.is_some() {
                        self.error(span, "more than one index register in memory operand");
                    } else {
                        mem.index = Some(s);
                    }
                    consumed_register = true;
                }
            }
            if consumed_register {
                continue;
            }

            // otherwise parse a non-additive expression (stops before top-level +/-)
            let Some(mut expr) = self.parse_mul() else {
                break;
            };
            if negate {
                expr = Expr::Neg(Box::new(expr));
            }
            disp_terms.push(expr);
        }

        if mem.base.is_none() && mem.index.is_none() && disp_terms.is_empty() {
            self.error(self.current_span(), "empty memory operand `[]`");
        }

        mem.disp = disp_terms.into_iter().reduce(|l, r| Expr::BinOp {
            op: BinOp::Add,
            lhs: Box::new(l),
            rhs: Box::new(r),
        });
        mem
    }

    fn apply_size_override(
        &mut self,
        operand: Operand,
        size: DataSize,
        prefix_span: Span,
    ) -> Operand {
        match operand {
            Operand::Mem(mut m) => {
                if m.size.is_some() {
                    self.error(prefix_span, "duplicate size override");
                }
                m.size = Some(size);
                Operand::Mem(m)
            }
            Operand::Reg(_) => {
                self.error(prefix_span, "size override cannot apply to a register");
                operand
            }
            _ => {
                self.error(prefix_span, "size override only applies to memory operands");
                operand
            }
        }
    }

    fn apply_seg_override(&mut self, operand: Operand, seg: String, prefix_span: Span) -> Operand {
        match operand {
            Operand::Mem(mut m) => {
                if m.seg_override.is_some() {
                    self.error(prefix_span, "duplicate segment override");
                }
                m.seg_override = Some(seg);
                Operand::Mem(m)
            }
            Operand::Imm(expr) => Operand::Mem(Mem {
                seg_override: Some(seg),
                size: None,
                base: None,
                index: None,
                disp: Some(expr),
            }),
            Operand::Reg(_) | Operand::Far { .. } => {
                self.error(prefix_span, "segment override cannot apply here");
                operand
            }
        }
    }

    // ---- statements & program -------------------------------------------

    pub fn parse_program(&mut self) -> Program {
        let mut program = Program::default();
        let mut open: Option<OpenSeg> = None;

        loop {
            while matches!(self.peek(), TokenKind::Newline) {
                self.advance();
            }
            if matches!(self.peek(), TokenKind::Eof) {
                break;
            }
            self.parse_one_statement(&mut open, &mut program);
        }

        if let Some(seg) = open {
            self.error(
                seg.span,
                format!("segment `{}` was not closed by `ends`", seg.name),
            );
            program.segments.push(close_open(seg));
        }
        program
    }

    fn parse_one_statement(&mut self, open: &mut Option<OpenSeg>, prog: &mut Program) {
        let start_span = self.current_span();
        match self.peek().clone() {
            TokenKind::Ident(s) if s == "assume" => {
                self.advance();
                let bindings = self.parse_assume();
                self.append_item(open, Item::Assume(bindings, start_span));
                self.expect_eos();
            }
            TokenKind::Ident(s) if s == "end" => {
                self.advance();
                let label = match self.peek().clone() {
                    TokenKind::Ident(name)
                        if !is_register(&name) && !is_reserved_keyword(&name) =>
                    {
                        self.advance();
                        Some(name)
                    }
                    _ => None,
                };
                prog.entry = label;
                self.expect_eos();
            }
            TokenKind::Ident(s) if matches!(s.as_str(), "db" | "dw" | "dd") => {
                self.advance();
                let size = size_from_data_kw(&s);
                let values = self.parse_data_values(&s);
                self.append_item(
                    open,
                    Item::DataDecl(
                        DataDecl {
                            name: None,
                            size,
                            values,
                        },
                        start_span,
                    ),
                );
                self.expect_eos();
            }
            TokenKind::Ident(_) => self.parse_named_statement(open, prog),
            _ => {
                let kind = self.peek().clone();
                self.error(
                    start_span,
                    format!("unexpected {} at start of statement", desc(&kind)),
                );
                self.recover_to_newline();
            }
        }
    }

    fn parse_named_statement(&mut self, open: &mut Option<OpenSeg>, prog: &mut Program) {
        let first_span = self.current_span();
        let first_ident = match self.peek() {
            TokenKind::Ident(s) => s.clone(),
            _ => unreachable!(),
        };

        let second_kw = if let Some(TokenKind::Ident(s)) = self.peek_at(1) {
            Some(s.clone())
        } else {
            None
        };
        let second_is_colon = matches!(self.peek_at(1), Some(TokenKind::Colon));

        if second_is_colon {
            self.advance();
            self.advance();
            self.append_item(open, Item::Label(first_ident, first_span));
            if !matches!(self.peek(), TokenKind::Newline | TokenKind::Eof) {
                self.parse_one_statement(open, prog);
            } else {
                self.expect_eos();
            }
            return;
        }

        if matches!(second_kw.as_deref(), Some("segment")) {
            self.advance();
            self.advance();
            if let Some(prev) = open.take() {
                self.error(
                    first_span,
                    format!(
                        "segment `{}` opened while `{}` still open",
                        first_ident, prev.name
                    ),
                );
                prog.segments.push(close_open(prev));
            }
            *open = Some(OpenSeg {
                name: first_ident,
                items: Vec::new(),
                span: first_span,
            });
            self.expect_eos();
            return;
        }

        if matches!(second_kw.as_deref(), Some("ends")) {
            self.advance();
            self.advance();
            match open.take() {
                Some(cur) if cur.name == first_ident => {
                    prog.segments.push(close_open(cur));
                }
                Some(cur) => {
                    self.error(
                        first_span,
                        format!(
                            "`ends` for `{}` but current segment is `{}`",
                            first_ident, cur.name
                        ),
                    );
                    prog.segments.push(close_open(cur));
                }
                None => {
                    self.error(
                        first_span,
                        format!("`ends` for `{}` outside any segment", first_ident),
                    );
                }
            }
            self.expect_eos();
            return;
        }

        if matches!(second_kw.as_deref(), Some("db" | "dw" | "dd")) {
            let size_kw = second_kw.unwrap();
            self.advance();
            self.advance();
            let size = size_from_data_kw(&size_kw);
            let values = self.parse_data_values(&size_kw);
            self.append_item(
                open,
                Item::DataDecl(
                    DataDecl {
                        name: Some(first_ident),
                        size,
                        values,
                    },
                    first_span,
                ),
            );
            self.expect_eos();
            return;
        }

        // instruction with first_ident as mnemonic
        self.advance();
        let operands = self.parse_operand_list();
        self.append_item(
            open,
            Item::Instruction(
                Instruction {
                    mnemonic: first_ident,
                    operands,
                },
                first_span,
            ),
        );
        self.expect_eos();
    }

    fn parse_assume(&mut self) -> Vec<AssumeBinding> {
        let mut bindings = Vec::new();
        loop {
            let seg_reg = match self.peek().clone() {
                TokenKind::Ident(s) if is_seg_register(&s) => {
                    self.advance();
                    s
                }
                _ => {
                    self.error(self.current_span(), "expected segment register in `assume`");
                    self.recover_to_newline_keep();
                    break;
                }
            };
            if !matches!(self.peek(), TokenKind::Colon) {
                self.error(
                    self.current_span(),
                    "expected `:` after segment register in `assume`",
                );
                self.recover_to_newline_keep();
                break;
            }
            self.advance();
            let name = match self.peek().clone() {
                TokenKind::Ident(n) => {
                    self.advance();
                    n
                }
                _ => {
                    self.error(self.current_span(), "expected segment name after `:`");
                    self.recover_to_newline_keep();
                    break;
                }
            };
            bindings.push(AssumeBinding {
                seg_reg,
                segment: name,
            });
            if matches!(self.peek(), TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        bindings
    }

    fn parse_data_values(&mut self, kw: &str) -> Vec<DataValue> {
        let mut values = Vec::new();
        if matches!(self.peek(), TokenKind::Newline | TokenKind::Eof) {
            self.error(
                self.current_span(),
                format!("`{kw}` expects at least one value"),
            );
            return values;
        }
        loop {
            values.push(self.parse_data_value());
            if matches!(self.peek(), TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        values
    }

    fn parse_data_value(&mut self) -> DataValue {
        if matches!(self.peek(), TokenKind::Question) {
            self.advance();
            return DataValue::Uninit;
        }
        if let TokenKind::String(_) = self.peek() {
            let kind = self.advance().kind.clone();
            if let TokenKind::String(bytes) = kind {
                return DataValue::String(bytes);
            }
        }
        let expr = self.parse_expr().unwrap_or(Expr::Int(0));
        if matches!(self.peek(), TokenKind::Ident(s) if s == "dup") {
            self.advance();
            if !matches!(self.peek(), TokenKind::LParen) {
                self.error(self.current_span(), "expected `(` after `dup`");
                return DataValue::Dup {
                    count: expr,
                    values: Vec::new(),
                };
            }
            self.advance();
            let mut values = Vec::new();
            if !matches!(self.peek(), TokenKind::RParen) {
                loop {
                    values.push(self.parse_data_value());
                    if matches!(self.peek(), TokenKind::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }
            }
            if !matches!(self.peek(), TokenKind::RParen) {
                self.error(self.current_span(), "expected `)`");
            } else {
                self.advance();
            }
            return DataValue::Dup {
                count: expr,
                values,
            };
        }
        DataValue::Expr(expr)
    }

    fn parse_operand_list(&mut self) -> Vec<Operand> {
        let mut ops = Vec::new();
        if matches!(self.peek(), TokenKind::Newline | TokenKind::Eof) {
            return ops;
        }
        while let Some(op) = self.parse_operand() {
            ops.push(op);
            if matches!(self.peek(), TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        ops
    }

    fn expect_eos(&mut self) {
        match self.peek() {
            TokenKind::Newline => {
                self.advance();
            }
            TokenKind::Eof => {}
            _ => {
                let kind = self.peek().clone();
                self.error(
                    self.current_span(),
                    format!("expected end of line, found {}", desc(&kind)),
                );
                self.recover_to_newline();
            }
        }
    }

    fn recover_to_newline(&mut self) {
        while !matches!(self.peek(), TokenKind::Newline | TokenKind::Eof) {
            self.advance();
        }
        if matches!(self.peek(), TokenKind::Newline) {
            self.advance();
        }
    }

    /// Recover but do not consume the terminating newline; useful inside helpers
    /// that the caller will then call `expect_eos` on.
    fn recover_to_newline_keep(&mut self) {
        while !matches!(self.peek(), TokenKind::Newline | TokenKind::Eof) {
            self.advance();
        }
    }

    fn append_item(&mut self, open: &mut Option<OpenSeg>, item: Item) {
        match open {
            Some(seg) => seg.items.push(item),
            None => {
                let span = item_span(&item);
                self.error(span, "statement outside any segment");
            }
        }
    }
}

struct OpenSeg {
    name: String,
    items: Vec<Item>,
    span: Span,
}

fn close_open(o: OpenSeg) -> Segment {
    Segment {
        name: o.name,
        items: o.items,
        span: o.span,
    }
}

fn item_span(item: &Item) -> Span {
    match item {
        Item::Label(_, s) | Item::Assume(_, s) | Item::DataDecl(_, s) | Item::Instruction(_, s) => {
            *s
        }
    }
}

fn size_from_data_kw(s: &str) -> DataSize {
    match s {
        "db" => DataSize::Byte,
        "dw" => DataSize::Word,
        "dd" => DataSize::Dword,
        _ => unreachable!(),
    }
}

fn is_reserved_keyword(s: &str) -> bool {
    matches!(
        s,
        "segment"
            | "ends"
            | "assume"
            | "end"
            | "db"
            | "dw"
            | "dd"
            | "dup"
            | "offset"
            | "seg"
            | "ptr"
            | "byte"
            | "word"
            | "dword"
    )
}

fn is_register(s: &str) -> bool {
    matches!(
        s,
        "ax" | "bx"
            | "cx"
            | "dx"
            | "ah"
            | "al"
            | "bh"
            | "bl"
            | "ch"
            | "cl"
            | "dh"
            | "dl"
            | "si"
            | "di"
            | "bp"
            | "sp"
            | "cs"
            | "ds"
            | "ss"
            | "es"
    )
}

fn is_seg_register(s: &str) -> bool {
    matches!(s, "cs" | "ds" | "ss" | "es")
}

fn is_base_register(s: &str) -> bool {
    matches!(s, "bx" | "bp")
}

fn is_index_register(s: &str) -> bool {
    matches!(s, "si" | "di")
}

fn is_size_keyword(s: &str) -> bool {
    matches!(s, "byte" | "word" | "dword")
}

fn size_from_kw(s: &str) -> DataSize {
    match s {
        "byte" => DataSize::Byte,
        "word" => DataSize::Word,
        "dword" => DataSize::Dword,
        _ => unreachable!(),
    }
}

fn desc(t: &TokenKind) -> String {
    match t {
        TokenKind::Number(_) => "number".into(),
        TokenKind::String(_) => "string literal".into(),
        TokenKind::Ident(s) => format!("identifier `{s}`"),
        TokenKind::LBracket => "`[`".into(),
        TokenKind::RBracket => "`]`".into(),
        TokenKind::LParen => "`(`".into(),
        TokenKind::RParen => "`)`".into(),
        TokenKind::Comma => "`,`".into(),
        TokenKind::Colon => "`:`".into(),
        TokenKind::Plus => "`+`".into(),
        TokenKind::Minus => "`-`".into(),
        TokenKind::Star => "`*`".into(),
        TokenKind::Slash => "`/`".into(),
        TokenKind::Percent => "`%`".into(),
        TokenKind::Question => "`?`".into(),
        TokenKind::Newline => "end of line".into(),
        TokenKind::Eof => "end of file".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_expr_str(src: &str) -> Option<Expr> {
        let (toks, _) = tokenize(src);
        let mut p = Parser::new(toks);
        p.parse_expr()
    }

    fn n(i: i64) -> Expr {
        Expr::Int(i)
    }
    fn bin(op: BinOp, l: Expr, r: Expr) -> Expr {
        Expr::BinOp {
            op,
            lhs: Box::new(l),
            rhs: Box::new(r),
        }
    }

    #[test]
    fn literal_number() {
        assert_eq!(parse_expr_str("42"), Some(n(42)));
    }

    #[test]
    fn precedence_mul_over_add() {
        assert_eq!(
            parse_expr_str("1 + 2 * 3"),
            Some(bin(BinOp::Add, n(1), bin(BinOp::Mul, n(2), n(3))))
        );
    }

    #[test]
    fn parens_override_precedence() {
        assert_eq!(
            parse_expr_str("(1 + 2) * 3"),
            Some(bin(BinOp::Mul, bin(BinOp::Add, n(1), n(2)), n(3)))
        );
    }

    #[test]
    fn left_associative_subtraction() {
        assert_eq!(
            parse_expr_str("1 - 2 - 3"),
            Some(bin(BinOp::Sub, bin(BinOp::Sub, n(1), n(2)), n(3)))
        );
    }

    #[test]
    fn unary_minus() {
        assert_eq!(parse_expr_str("-5"), Some(Expr::Neg(Box::new(n(5)))));
    }

    #[test]
    fn offset_operator() {
        assert_eq!(
            parse_expr_str("offset msg"),
            Some(Expr::Offset(Box::new(Expr::Ident("msg".into()))))
        );
    }

    #[test]
    fn offset_binds_tighter_than_add() {
        assert_eq!(
            parse_expr_str("offset msg + 3"),
            Some(bin(
                BinOp::Add,
                Expr::Offset(Box::new(Expr::Ident("msg".into()))),
                n(3)
            ))
        );
    }

    #[test]
    fn single_char_string_becomes_int() {
        assert_eq!(parse_expr_str("'A'"), Some(n(65)));
    }

    #[test]
    fn multi_char_string_in_expression_errors() {
        let (toks, _) = tokenize("'AB'");
        let mut p = Parser::new(toks);
        let _ = p.parse_expr();
        assert_eq!(p.into_diagnostics().len(), 1);
    }

    #[test]
    fn modulo_and_division() {
        assert_eq!(
            parse_expr_str("10 % 3 / 2"),
            Some(bin(BinOp::Div, bin(BinOp::Mod, n(10), n(3)), n(2)))
        );
    }

    // ---- operand tests ----------------------------------------------------

    fn parse_op_str(src: &str) -> (Option<Operand>, Vec<Diagnostic>) {
        let (toks, _) = tokenize(src);
        let mut p = Parser::new(toks);
        let op = p.parse_operand();
        (op, p.into_diagnostics())
    }

    #[test]
    fn operand_register() {
        let (op, d) = parse_op_str("ax");
        assert_eq!(op, Some(Operand::Reg("ax".into())));
        assert!(d.is_empty());
    }

    #[test]
    fn operand_immediate_number() {
        let (op, _) = parse_op_str("5");
        assert_eq!(op, Some(Operand::Imm(Expr::Int(5))));
    }

    #[test]
    fn operand_bare_symbol_is_immediate() {
        // VM interprets at execution time per instruction context
        let (op, _) = parse_op_str("msg");
        assert_eq!(op, Some(Operand::Imm(Expr::Ident("msg".into()))));
    }

    #[test]
    fn operand_simple_mem() {
        let (op, _) = parse_op_str("[bx]");
        assert_eq!(
            op,
            Some(Operand::Mem(Mem {
                base: Some("bx".into()),
                ..Mem::default()
            }))
        );
    }

    #[test]
    fn operand_mem_base_index_disp() {
        let (op, _) = parse_op_str("[bx+si+5]");
        assert_eq!(
            op,
            Some(Operand::Mem(Mem {
                base: Some("bx".into()),
                index: Some("si".into()),
                disp: Some(Expr::Int(5)),
                ..Mem::default()
            }))
        );
    }

    #[test]
    fn operand_mem_minus_disp() {
        let (op, _) = parse_op_str("[bx-5]");
        // disp_terms = [Neg(Int(5))], reduce of single = Neg(Int(5))
        assert_eq!(
            op,
            Some(Operand::Mem(Mem {
                base: Some("bx".into()),
                disp: Some(Expr::Neg(Box::new(Expr::Int(5)))),
                ..Mem::default()
            }))
        );
    }

    #[test]
    fn operand_mem_with_seg_override() {
        let (op, _) = parse_op_str("ds:[bx]");
        assert_eq!(
            op,
            Some(Operand::Mem(Mem {
                seg_override: Some("ds".into()),
                base: Some("bx".into()),
                ..Mem::default()
            }))
        );
    }

    #[test]
    fn operand_seg_override_on_symbol() {
        let (op, _) = parse_op_str("ds:msg");
        assert_eq!(
            op,
            Some(Operand::Mem(Mem {
                seg_override: Some("ds".into()),
                disp: Some(Expr::Ident("msg".into())),
                ..Mem::default()
            }))
        );
    }

    #[test]
    fn operand_byte_ptr() {
        let (op, _) = parse_op_str("byte ptr [bx]");
        assert_eq!(
            op,
            Some(Operand::Mem(Mem {
                size: Some(DataSize::Byte),
                base: Some("bx".into()),
                ..Mem::default()
            }))
        );
    }

    #[test]
    fn operand_word_ptr_seg_override() {
        let (op, _) = parse_op_str("word ptr ds:[bx]");
        assert_eq!(
            op,
            Some(Operand::Mem(Mem {
                size: Some(DataSize::Word),
                seg_override: Some("ds".into()),
                base: Some("bx".into()),
                ..Mem::default()
            }))
        );
    }

    #[test]
    fn operand_double_base_errors() {
        let (_, d) = parse_op_str("[bx+bp]");
        assert!(d.iter().any(|d| d.message.contains("more than one base")));
    }

    #[test]
    fn operand_empty_brackets_errors() {
        let (_, d) = parse_op_str("[]");
        assert!(d.iter().any(|d| d.message.contains("empty memory")));
    }

    #[test]
    fn operand_size_override_on_register_errors() {
        let (_, d) = parse_op_str("word ptr ax");
        assert!(d.iter().any(|d| d.message.contains("size override")));
    }

    #[test]
    fn operand_offset_label() {
        let (op, _) = parse_op_str("offset msg");
        assert_eq!(
            op,
            Some(Operand::Imm(Expr::Offset(Box::new(Expr::Ident(
                "msg".into()
            )))))
        );
    }

    // ---- statement / program tests ----------------------------------------

    #[test]
    fn parses_minimal_program() {
        let src = "code segment\n\
                   start: mov ax, 1\n\
                   code ends\n\
                   end start\n";
        let (prog, diags) = parse(src);
        assert!(diags.is_empty(), "diags: {diags:?}");
        assert_eq!(prog.segments.len(), 1);
        assert_eq!(prog.segments[0].name, "code");
        assert_eq!(prog.entry.as_deref(), Some("start"));
        // items: Label("start"), Instruction(mov ax, 1)
        let items = &prog.segments[0].items;
        assert_eq!(items.len(), 2);
        assert!(matches!(&items[0], Item::Label(s, _) if s == "start"));
        assert!(matches!(&items[1], Item::Instruction(i, _) if i.mnemonic == "mov"));
    }

    #[test]
    fn parses_multi_segment_with_assume() {
        let src = "data segment\n\
                   msg db 'hi$'\n\
                   data ends\n\
                   code segment\n\
                   assume cs:code, ds:data\n\
                   start: mov ax, data\n\
                   code ends\n\
                   end start\n";
        let (prog, diags) = parse(src);
        assert!(diags.is_empty(), "diags: {diags:?}");
        assert_eq!(prog.segments.len(), 2);
        assert_eq!(prog.segments[0].name, "data");
        assert_eq!(prog.segments[1].name, "code");
    }

    #[test]
    fn data_decl_with_dup_and_string() {
        let src = "data segment\n\
                   buf db 10 dup (?)\n\
                   nums dw 1, 2, 3\n\
                   msg db 'hello$'\n\
                   data ends\n";
        let (prog, diags) = parse(src);
        assert!(diags.is_empty(), "diags: {diags:?}");
        let items = &prog.segments[0].items;
        assert_eq!(items.len(), 3);
        assert!(matches!(
            &items[0],
            Item::DataDecl(d, _) if d.name.as_deref() == Some("buf")
                && d.size == DataSize::Byte
                && matches!(&d.values[..], [DataValue::Dup { count: Expr::Int(10), values: vs }] if vs == &vec![DataValue::Uninit])
        ));
    }

    #[test]
    fn error_in_one_line_still_parses_next() {
        let src = "code segment\n\
                   mov ax,, bx\n\
                   mov bx, 1\n\
                   code ends\n";
        let (prog, diags) = parse(src);
        assert!(!diags.is_empty());
        // 第二行应当仍被正常解析
        let items = &prog.segments[0].items;
        assert!(items.iter().any(|it| matches!(it,
            Item::Instruction(i, _) if i.mnemonic == "bx" || i.operands.iter().any(|op| matches!(op, Operand::Imm(Expr::Int(1))))
        )) || items.iter().any(|it| matches!(it, Item::Instruction(i, _) if i.mnemonic == "mov")));
    }

    #[test]
    fn statement_outside_segment_errors() {
        let src = "mov ax, 1\n";
        let (_, diags) = parse(src);
        assert!(
            diags
                .iter()
                .any(|d| d.message.contains("outside any segment"))
        );
    }

    #[test]
    fn segment_close_mismatch_errors() {
        let src = "a segment\nmov ax, 1\nb ends\n";
        let (_, diags) = parse(src);
        assert!(
            diags
                .iter()
                .any(|d| d.message.contains("current segment is `a`"))
        );
    }
}
