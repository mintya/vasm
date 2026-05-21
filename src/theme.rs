//! 终端配色主题。默认值是 "DOS 绿" 风格（仿 Borland Turbo C），
//! 可从 `$XDG_CONFIG_HOME/vasm/config.toml` 加载覆写。
//!
//! 不存在配置文件、解析失败、字段缺省都回退到默认——
//! 教学项目的可用性优先，不让配置错误阻塞启动。

use ratatui::style::Color;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct Theme {
    pub border: Color,
    pub border_focused: Color,
    pub outer: Color,
    pub console_bg: Color,
    pub console_output: Color,
    pub console_echo: Color,
    pub console_cursor: Color,
    pub register_name: Color,
    pub register_value: Color,
    pub flag_set: Color,
    pub flag_clear: Color,
    pub status_paused: Color,
    pub status_halted: Color,
    pub status_error: Color,
    pub status_waiting: Color,
    pub explain: Color,
    pub muted: Color,
    pub source_keyword: Color,
    pub source_number: Color,
    pub source_string: Color,
    pub source_register: Color,
    pub source_pc: Color,
    pub source_breakpoint: Color,
    pub prompt_border: Color,
}

impl Default for Theme {
    fn default() -> Self {
        // 兼容 M0-M5 既有视觉：保持 "DOS 绿" 调
        Self {
            border: Color::Cyan,
            border_focused: Color::Cyan,
            outer: Color::Blue,
            console_bg: Color::Black,
            console_output: Color::Green,
            console_echo: Color::Yellow,
            console_cursor: Color::Cyan,
            register_name: Color::Gray,
            register_value: Color::White,
            flag_set: Color::Green,
            flag_clear: Color::DarkGray,
            status_paused: Color::Yellow,
            status_halted: Color::Green,
            status_error: Color::Red,
            status_waiting: Color::Magenta,
            explain: Color::Gray,
            muted: Color::DarkGray,
            source_keyword: Color::Yellow,
            source_number: Color::Green,
            source_string: Color::Yellow,
            source_register: Color::Cyan,
            source_pc: Color::Yellow,
            source_breakpoint: Color::Red,
            prompt_border: Color::Magenta,
        }
    }
}

impl Theme {
    /// 尝试从 `~/.config/vasm/config.toml` 加载；任何失败回退默认。
    pub fn load_or_default() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::default();
        };
        let Ok(text) = std::fs::read_to_string(&path) else {
            return Self::default();
        };
        match toml::from_str::<FileConfig>(&text) {
            Ok(cfg) => cfg.theme.unwrap_or_default().merged_into(Self::default()),
            Err(e) => {
                tracing::warn!("failed to parse {}: {e}", path.display());
                Self::default()
            }
        }
    }

    /// 显式从字符串加载（测试用）。
    pub fn from_toml_str(text: &str) -> Self {
        match toml::from_str::<FileConfig>(text) {
            Ok(cfg) => cfg.theme.unwrap_or_default().merged_into(Self::default()),
            Err(_) => Self::default(),
        }
    }

    fn config_path() -> Option<std::path::PathBuf> {
        dirs::config_dir().map(|d| d.join("vasm").join("config.toml"))
    }
}

#[derive(Debug, Default, Deserialize)]
struct FileConfig {
    theme: Option<ThemeOverride>,
}

/// 配置文件里 [theme] 节的可选字段。任何字段缺省 = 用 default 值。
#[derive(Debug, Default, Deserialize)]
struct ThemeOverride {
    border: Option<String>,
    border_focused: Option<String>,
    outer: Option<String>,
    console_bg: Option<String>,
    console_output: Option<String>,
    console_echo: Option<String>,
    console_cursor: Option<String>,
    register_name: Option<String>,
    register_value: Option<String>,
    flag_set: Option<String>,
    flag_clear: Option<String>,
    status_paused: Option<String>,
    status_halted: Option<String>,
    status_error: Option<String>,
    status_waiting: Option<String>,
    explain: Option<String>,
    muted: Option<String>,
    source_keyword: Option<String>,
    source_number: Option<String>,
    source_string: Option<String>,
    source_register: Option<String>,
    source_pc: Option<String>,
    source_breakpoint: Option<String>,
    prompt_border: Option<String>,
}

impl ThemeOverride {
    fn merged_into(self, mut base: Theme) -> Theme {
        macro_rules! apply {
            ($field:ident) => {
                if let Some(v) = self.$field.as_deref().and_then(parse_color) {
                    base.$field = v;
                }
            };
        }
        apply!(border);
        apply!(border_focused);
        apply!(outer);
        apply!(console_bg);
        apply!(console_output);
        apply!(console_echo);
        apply!(console_cursor);
        apply!(register_name);
        apply!(register_value);
        apply!(flag_set);
        apply!(flag_clear);
        apply!(status_paused);
        apply!(status_halted);
        apply!(status_error);
        apply!(status_waiting);
        apply!(explain);
        apply!(muted);
        apply!(source_keyword);
        apply!(source_number);
        apply!(source_string);
        apply!(source_register);
        apply!(source_pc);
        apply!(source_breakpoint);
        apply!(prompt_border);
        base
    }
}

/// 解析颜色字符串：支持命名色（"red" / "darkgray" / ...）和 hex（"#RRGGBB"）。
fn parse_color(s: &str) -> Option<Color> {
    let t = s.trim();
    // hex
    if let Some(hex) = t.strip_prefix('#')
        && hex.len() == 6
        && let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
        )
    {
        return Some(Color::Rgb(r, g, b));
    }
    Some(match t.to_ascii_lowercase().as_str() {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" | "grey" => Color::Gray,
        "darkgray" | "darkgrey" => Color::DarkGray,
        "lightred" => Color::LightRed,
        "lightgreen" => Color::LightGreen,
        "lightyellow" => Color::LightYellow,
        "lightblue" => Color::LightBlue,
        "lightmagenta" => Color::LightMagenta,
        "lightcyan" => Color::LightCyan,
        "white" => Color::White,
        "reset" => Color::Reset,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_keeps_dos_palette() {
        let t = Theme::default();
        assert_eq!(t.console_output, Color::Green);
        assert_eq!(t.status_paused, Color::Yellow);
    }

    #[test]
    fn override_one_field_keeps_others() {
        let t = Theme::from_toml_str(
            r##"[theme]
console_output = "lightgreen"
"##,
        );
        assert_eq!(t.console_output, Color::LightGreen);
        assert_eq!(t.status_paused, Color::Yellow);
    }

    #[test]
    fn hex_color_parses() {
        let t = Theme::from_toml_str(
            r##"[theme]
border = "#ff8800"
"##,
        );
        assert_eq!(t.border, Color::Rgb(0xFF, 0x88, 0x00));
    }

    #[test]
    fn bad_color_falls_back_to_default() {
        let t = Theme::from_toml_str(
            r##"[theme]
console_output = "not-a-color"
"##,
        );
        assert_eq!(t.console_output, Color::Green);
    }

    #[test]
    fn invalid_toml_falls_back_to_default() {
        let t = Theme::from_toml_str("this is not toml [[");
        assert_eq!(t.console_output, Color::Green);
    }
}
