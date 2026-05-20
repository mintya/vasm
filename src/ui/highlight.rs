use ratatui::style::{Color, Style};
use ratatui::text::Span;

const REGS: &[&str] = &[
    "ax", "bx", "cx", "dx", "ah", "al", "bh", "bl", "ch", "cl", "dh", "dl", "si", "di", "bp", "sp",
    "cs", "ds", "ss", "es", "ip",
];

/// 把单行汇编源码切成带样式的 spans。不依赖完整 lexer，逐字符扫描即可。
pub fn highlight_line(line: &str) -> Vec<Span<'static>> {
    let mut out: Vec<Span<'static>> = Vec::new();
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b';' {
            out.push(Span::styled(
                line[i..].to_string(),
                Style::default().fg(Color::DarkGray),
            ));
            return out;
        }
        if b == b'\'' || b == b'"' {
            let quote = b;
            let start = i;
            i += 1;
            while i < bytes.len() && bytes[i] != quote {
                i += 1;
            }
            if i < bytes.len() {
                i += 1; // 闭合引号
            }
            out.push(Span::styled(
                line[start..i].to_string(),
                Style::default().fg(Color::Yellow),
            ));
            continue;
        }
        if b.is_ascii_digit() {
            let start = i;
            while i < bytes.len() && is_num_body(bytes[i]) {
                i += 1;
            }
            out.push(Span::styled(
                line[start..i].to_string(),
                Style::default().fg(Color::Green),
            ));
            continue;
        }
        if is_ident_start(b) {
            let start = i;
            while i < bytes.len() && is_ident_body(bytes[i]) {
                i += 1;
            }
            let word = &line[start..i];
            let lower = word.to_ascii_lowercase();
            let style = if REGS.contains(&lower.as_str()) {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            };
            out.push(Span::styled(word.to_string(), style));
            continue;
        }
        // 单字符（标点、空白等），保持默认样式但单独成 span 以避免与上一 span 混淆。
        out.push(Span::raw(line[i..i + 1].to_string()));
        i += 1;
    }
    out
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b == b'.' || b == b'@' || b == b'$'
}

fn is_ident_body(b: u8) -> bool {
    is_ident_start(b) || b.is_ascii_digit()
}

fn is_num_body(b: u8) -> bool {
    b.is_ascii_hexdigit() || b == b'H' || b == b'h' || b == b'B' || b == b'b'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comment_styled_gray() {
        let spans = highlight_line("  mov ax, 1  ; comment");
        let last = spans.last().unwrap();
        assert!(last.content.starts_with("; comment"));
        assert_eq!(last.style.fg, Some(Color::DarkGray));
    }

    #[test]
    fn registers_cyan() {
        let spans = highlight_line("mov ax, bx");
        let ax = spans.iter().find(|s| s.content == "ax").unwrap();
        assert_eq!(ax.style.fg, Some(Color::Cyan));
        let bx = spans.iter().find(|s| s.content == "bx").unwrap();
        assert_eq!(bx.style.fg, Some(Color::Cyan));
    }

    #[test]
    fn numbers_green() {
        let spans = highlight_line("mov ax, 1234h");
        let num = spans.iter().find(|s| s.content == "1234h").unwrap();
        assert_eq!(num.style.fg, Some(Color::Green));
    }

    #[test]
    fn string_yellow() {
        let spans = highlight_line("db 'hi'");
        let s = spans.iter().find(|s| s.content == "'hi'").unwrap();
        assert_eq!(s.style.fg, Some(Color::Yellow));
    }
}
