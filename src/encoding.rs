//! Console 字符编码（DOS 字节流 ↔ Unicode）。
//!
//! - `Utf8`：直通；
//! - `Gbk`：用 `encoding_rs` 的 GBK；适合王爽教材中文字符串；
//! - `Cp437`：DOS 原生 OEM 编码，含 box drawing 字符；encoding_rs 不含此编码，
//!   这里用一张 256 项的码点表手工映射（仅 BMP，输入侧反查表）。

use clap::ValueEnum;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "lower")]
pub enum Encoding {
    Utf8,
    Gbk,
    Cp437,
}

impl Encoding {
    /// 把 DOS 字节流解码成 UTF-8 字符串用于渲染。非法字节用 U+FFFD 替换。
    pub fn decode(self, bytes: &[u8]) -> String {
        match self {
            Encoding::Utf8 => String::from_utf8_lossy(bytes).into_owned(),
            Encoding::Gbk => {
                let (cow, _enc, _had_errors) = encoding_rs::GBK.decode(bytes);
                cow.into_owned()
            }
            Encoding::Cp437 => bytes.iter().map(|&b| CP437[b as usize]).collect(),
        }
    }

    /// 把用户敲入的字符按本编码编码成字节序列追加到 `out`。
    /// 编码失败的字符（如 CP437 不含的 emoji）忽略——教学场景输入主要是 ASCII/中文。
    pub fn encode_char(self, c: char, out: &mut Vec<u8>) {
        match self {
            Encoding::Utf8 => {
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                out.extend_from_slice(s.as_bytes());
            }
            Encoding::Gbk => {
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                let (cow, _enc, had_errors) = encoding_rs::GBK.encode(s);
                if !had_errors {
                    out.extend_from_slice(&cow);
                }
            }
            Encoding::Cp437 => {
                if let Some(b) = cp437_encode(c) {
                    out.push(b);
                }
            }
        }
    }
}

fn cp437_encode(c: char) -> Option<u8> {
    CP437.iter().position(|&ch| ch == c).map(|i| i as u8)
}

/// IBM PC code page 437 → Unicode BMP. Reference: https://en.wikipedia.org/wiki/Code_page_437
#[rustfmt::skip]
const CP437: [char; 256] = [
    // 0x00-0x1F: 控制字符区域 CP437 也定义了可打印字形（笑脸、扑克花色等）
    '\u{0000}', '\u{263A}', '\u{263B}', '\u{2665}', '\u{2666}', '\u{2663}', '\u{2660}', '\u{2022}',
    '\u{25D8}', '\u{25CB}', '\u{25D9}', '\u{2642}', '\u{2640}', '\u{266A}', '\u{266B}', '\u{263C}',
    '\u{25BA}', '\u{25C4}', '\u{2195}', '\u{203C}', '\u{00B6}', '\u{00A7}', '\u{25AC}', '\u{21A8}',
    '\u{2191}', '\u{2193}', '\u{2192}', '\u{2190}', '\u{221F}', '\u{2194}', '\u{25B2}', '\u{25BC}',
    // 0x20-0x7F: ASCII
    ' ', '!', '"', '#', '$', '%', '&', '\'', '(', ')', '*', '+', ',', '-', '.', '/',
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', ':', ';', '<', '=', '>', '?',
    '@', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O',
    'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '[', '\\', ']', '^', '_',
    '`', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o',
    'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '{', '|', '}', '~', '\u{2302}',
    // 0x80-0xFF: OEM 扩展
    'Ç', 'ü', 'é', 'â', 'ä', 'à', 'å', 'ç', 'ê', 'ë', 'è', 'ï', 'î', 'ì', 'Ä', 'Å',
    'É', 'æ', 'Æ', 'ô', 'ö', 'ò', 'û', 'ù', 'ÿ', 'Ö', 'Ü', '¢', '£', '¥', '₧', 'ƒ',
    'á', 'í', 'ó', 'ú', 'ñ', 'Ñ', 'ª', 'º', '¿', '⌐', '¬', '½', '¼', '¡', '«', '»',
    '░', '▒', '▓', '│', '┤', '╡', '╢', '╖', '╕', '╣', '║', '╗', '╝', '╜', '╛', '┐',
    '└', '┴', '┬', '├', '─', '┼', '╞', '╟', '╚', '╔', '╩', '╦', '╠', '═', '╬', '╧',
    '╨', '╤', '╥', '╙', '╘', '╒', '╓', '╫', '╪', '┘', '┌', '█', '▄', '▌', '▐', '▀',
    'α', 'ß', 'Γ', 'π', 'Σ', 'σ', 'µ', 'τ', 'Φ', 'Θ', 'Ω', 'δ', '∞', 'φ', 'ε', '∩',
    '≡', '±', '≥', '≤', '⌠', '⌡', '÷', '≈', '°', '∙', '·', '√', 'ⁿ', '²', '■', '\u{00A0}',
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_passthrough() {
        assert_eq!(Encoding::Utf8.decode(b"hi"), "hi");
    }

    #[test]
    fn gbk_decodes_chinese() {
        // 0xC4 0xE3 = "你"
        assert_eq!(Encoding::Gbk.decode(&[0xC4, 0xE3]), "你");
    }

    #[test]
    fn gbk_encodes_chinese_char() {
        let mut out = Vec::new();
        Encoding::Gbk.encode_char('你', &mut out);
        assert_eq!(out, vec![0xC4, 0xE3]);
    }

    #[test]
    fn cp437_decodes_box_char() {
        // 0xC4 = '─'（水平线）
        assert_eq!(Encoding::Cp437.decode(&[0xC4]), "─");
    }

    #[test]
    fn cp437_round_trip_ascii() {
        let bytes = b"ABC";
        let s = Encoding::Cp437.decode(bytes);
        assert_eq!(s, "ABC");
        let mut out = Vec::new();
        for c in s.chars() {
            Encoding::Cp437.encode_char(c, &mut out);
        }
        assert_eq!(out, bytes);
    }

    #[test]
    fn ascii_works_in_all_encodings() {
        for enc in [Encoding::Utf8, Encoding::Gbk, Encoding::Cp437] {
            assert_eq!(enc.decode(b"hello"), "hello");
        }
    }
}
