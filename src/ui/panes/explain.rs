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
            let head = if ops.is_empty() {
                format!("▶ {}", instr.mnemonic)
            } else {
                format!("▶ {} {}", instr.mnemonic, ops)
            };
            if let Some(note) = explain_int(app, instr) {
                format!("{head}    ; {note}")
            } else {
                head
            }
        }
        None => "▶ (halted)".to_string(),
    };
    let line = Line::from(Span::styled(text, Style::default().fg(Color::Gray)));
    Paragraph::new(line).render(area, buf);
}

/// 给 `int <n>` 配一段教学注释。已知 ah 时给出 stub 的语义；否则给通用 IVT 描述。
fn explain_int(app: &App, instr: &crate::asm::ast::Instruction) -> Option<String> {
    if instr.mnemonic != "int" || instr.operands.len() != 1 {
        return None;
    }
    let n = match &instr.operands[0] {
        Operand::Imm(Expr::Int(v)) => *v as u8,
        _ => return None,
    };
    let ah = app.vm().map(|vm| (vm.cpu.ax >> 8) as u8);
    Some(match (n, ah) {
        (0x21, Some(0x01)) => "DOS 01h: 阻塞读字符回显 → al".into(),
        (0x21, Some(0x02)) => "DOS 02h: 输出 dl 字符".into(),
        (0x21, Some(0x09)) => "DOS 09h: 输出 '$' 结尾字符串 ds:dx".into(),
        (0x21, Some(0x0A)) => "DOS 0Ah: 缓冲键盘输入到 ds:dx".into(),
        (0x21, Some(0x4C)) => "DOS 4Ch: 退出程序".into(),
        (0x21, Some(other)) => format!("DOS {other:02X}h"),
        (0x10, Some(0x02)) => "BIOS 10h 02h: 设光标位置 (dh,dl)".into(),
        (0x10, Some(0x09 | 0x0A)) => "BIOS 10h 09h: 重复输出 al, cx 次".into(),
        (0x10, Some(0x13)) => "BIOS 10h 13h: 写字符串 es:bp, cx 字节".into(),
        (0x16, Some(0x00)) => "BIOS 16h 00h: 阻塞读键 → al".into(),
        (0x16, Some(0x01)) => "BIOS 16h 01h: 非阻塞查键 (ZF=空)".into(),
        _ => format!("push flags; push cs; push ip; goto IVT[{n:02X}h]"),
    })
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
