use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::app::App;
use crate::asm::ast::{Expr, Mem, Operand};

pub fn render(area: Rect, buf: &mut Buffer, app: &App) {
    let text = match app.vm().and_then(|vm| vm.current_instruction()) {
        Some(instr) => {
            let ops = instr
                .operands
                .iter()
                .map(format_operand)
                .collect::<Vec<_>>()
                .join(", ");
            if ops.is_empty() {
                format!("▶ {}", instr.mnemonic)
            } else {
                format!("▶ {} {}", instr.mnemonic, ops)
            }
        }
        None => "▶ (halted)".to_string(),
    };
    let line = Line::from(Span::styled(text, Style::default().fg(Color::Gray)));
    Paragraph::new(line).render(area, buf);
}

fn format_operand(op: &Operand) -> String {
    match op {
        Operand::Reg(name) => name.clone(),
        Operand::Imm(e) => format_expr(e),
        Operand::Mem(m) => format_mem(m),
        Operand::Far { seg, off } => format!("{seg:04X}:{off:04X}"),
    }
}

fn format_mem(m: &Mem) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(base) = &m.base {
        parts.push(base.clone());
    }
    if let Some(idx) = &m.index {
        parts.push(idx.clone());
    }
    if let Some(disp) = &m.disp {
        parts.push(format_expr(disp));
    }
    let inner = parts.join("+");
    match &m.seg_override {
        Some(s) => format!("{s}:[{inner}]"),
        None => format!("[{inner}]"),
    }
}

fn format_expr(e: &Expr) -> String {
    match e {
        Expr::Int(n) => format!("{n}"),
        Expr::Ident(s) => s.clone(),
        Expr::Neg(inner) => format!("-{}", format_expr(inner)),
        Expr::BinOp { op, lhs, rhs } => {
            let sym = match op {
                crate::asm::ast::BinOp::Add => "+",
                crate::asm::ast::BinOp::Sub => "-",
                crate::asm::ast::BinOp::Mul => "*",
                crate::asm::ast::BinOp::Div => "/",
                crate::asm::ast::BinOp::Mod => "%",
            };
            format!("{}{}{}", format_expr(lhs), sym, format_expr(rhs))
        }
        Expr::Offset(inner) => format!("offset {}", format_expr(inner)),
        Expr::Seg(inner) => format!("seg {}", format_expr(inner)),
    }
}
