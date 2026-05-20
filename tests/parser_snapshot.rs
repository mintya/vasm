use std::path::Path;

use vasm::asm::parser::parse;

fn read_fixture(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

macro_rules! snap {
    ($name:ident, $file:expr) => {
        #[test]
        fn $name() {
            let src = read_fixture($file);
            let parsed = parse(&src);
            insta::assert_debug_snapshot!(parsed);
        }
    };
}

snap!(basic, "m1_basic.asm");
snap!(addressing, "m1_addressing.asm");
snap!(loop_chapter, "m1_loop.asm");
snap!(multi_segment, "m1_multi_segment.asm");
snap!(data, "m1_data.asm");
snap!(expr, "m1_expr.asm");
snap!(jumps, "m1_jumps.asm");
snap!(int_21h, "m1_int.asm");
snap!(errors, "m1_errors.asm");

#[test]
fn crlf_line_endings_are_accepted() {
    // 直接构造 CRLF，避免 git 跨平台行结束符转换的坑
    let src = "code segment\r\n  mov ax, 1\r\n  add ax, 2\r\ncode ends\r\nend\r\n";
    let parsed = parse(src);
    insta::assert_debug_snapshot!(parsed);
}
