//! 指令元数据表：助记符 → 一行教学摘要 + 多行 C 伪代码语义。
//!
//! Stage C 的 explain pane 用它代替原本写死的字符串。`lookup` 接收的助记符
//! 已被小写化（与 `dispatch` 一致），命中返回 `&'static InsnDoc`，未命中返 `None`。

#[derive(Debug, Clone, Copy)]
pub struct InsnDoc {
    pub mnemonic: &'static str,
    pub summary: &'static str,
    pub semantics: &'static str,
}

/// 按助记符查文档。`mnemonic` 应为小写。
pub fn lookup(mnemonic: &str) -> Option<&'static InsnDoc> {
    TABLE.iter().find(|d| d.mnemonic == mnemonic)
}

/// 内置静态表。覆盖教材常见 30+ 条；未列入的助记符返 None。
static TABLE: &[InsnDoc] = &[
    // ---- 数据传送 ----
    InsnDoc {
        mnemonic: "mov",
        summary: "把源操作数复制到目的（不改 flags）",
        semantics: "dst = src;",
    },
    InsnDoc {
        mnemonic: "xchg",
        summary: "交换两个操作数（不改 flags）",
        semantics: "tmp = dst; dst = src; src = tmp;",
    },
    // ---- 栈 ----
    InsnDoc {
        mnemonic: "push",
        summary: "压栈：sp -= 2; [ss:sp] = src",
        semantics: "sp -= 2;\nmem16[ss:sp] = src;",
    },
    InsnDoc {
        mnemonic: "pop",
        summary: "出栈：dst = [ss:sp]; sp += 2",
        semantics: "dst = mem16[ss:sp];\nsp += 2;",
    },
    InsnDoc {
        mnemonic: "pushf",
        summary: "把 flags 寄存器压栈",
        semantics: "sp -= 2; mem16[ss:sp] = flags;",
    },
    InsnDoc {
        mnemonic: "popf",
        summary: "从栈顶弹出一个字到 flags 寄存器",
        semantics: "flags = mem16[ss:sp]; sp += 2;",
    },
    // ---- 算术 ----
    InsnDoc {
        mnemonic: "add",
        summary: "整数加：dst += src；置 CF/ZF/SF/OF/AF/PF",
        semantics: "dst = dst + src;\nset_arith_flags(...);",
    },
    InsnDoc {
        mnemonic: "sub",
        summary: "整数减：dst -= src；置 CF/ZF/SF/OF/AF/PF",
        semantics: "dst = dst - src;\nset_arith_flags(...);",
    },
    InsnDoc {
        mnemonic: "inc",
        summary: "自增 1（不改 CF）",
        semantics: "dst += 1;",
    },
    InsnDoc {
        mnemonic: "dec",
        summary: "自减 1（不改 CF）",
        semantics: "dst -= 1;",
    },
    InsnDoc {
        mnemonic: "cmp",
        summary: "按 dst-src 设 flags，不写回",
        semantics: "discard = dst - src;\nset_arith_flags(...);",
    },
    InsnDoc {
        mnemonic: "neg",
        summary: "取负：dst = 0 - dst",
        semantics: "dst = -dst;",
    },
    InsnDoc {
        mnemonic: "mul",
        summary: "无符号乘 (ax / dx:ax)",
        semantics: "if size == 8: ax = al * src;\nelse: dx:ax = ax * src;",
    },
    InsnDoc {
        mnemonic: "div",
        summary: "无符号除：商进 al/ax，余数进 ah/dx",
        semantics: "if size==8: al=ax/src, ah=ax%src;\nelse: ax=dx:ax/src, dx=dx:ax%src;",
    },
    // ---- 逻辑 ----
    InsnDoc {
        mnemonic: "and",
        summary: "按位与；CF=OF=0，按结果置 ZF/SF/PF",
        semantics: "dst = dst & src;",
    },
    InsnDoc {
        mnemonic: "or",
        summary: "按位或；CF=OF=0",
        semantics: "dst = dst | src;",
    },
    InsnDoc {
        mnemonic: "xor",
        summary: "按位异或（xor reg,reg 是清零惯用法）",
        semantics: "dst = dst ^ src;",
    },
    InsnDoc {
        mnemonic: "not",
        summary: "按位反（不改 flags）",
        semantics: "dst = ~dst;",
    },
    InsnDoc {
        mnemonic: "test",
        summary: "按 dst & src 设 flags，不写回",
        semantics: "discard = dst & src;\nset_logic_flags(...);",
    },
    // ---- 移位 ----
    InsnDoc {
        mnemonic: "shl",
        summary: "逻辑左移 cl 位（=sal）",
        semantics: "dst <<= cnt;\nCF = msb_out;",
    },
    InsnDoc {
        mnemonic: "sal",
        summary: "算术左移（=shl）",
        semantics: "dst <<= cnt;",
    },
    InsnDoc {
        mnemonic: "shr",
        summary: "逻辑右移：高位补 0",
        semantics: "dst = (u16) dst >> cnt;",
    },
    InsnDoc {
        mnemonic: "sar",
        summary: "算术右移：高位补符号位",
        semantics: "dst = (i16) dst >> cnt;",
    },
    InsnDoc {
        mnemonic: "rol",
        summary: "循环左移（不经 CF）",
        semantics: "dst = (dst << cnt) | (dst >> (n-cnt));",
    },
    InsnDoc {
        mnemonic: "ror",
        summary: "循环右移",
        semantics: "dst = (dst >> cnt) | (dst << (n-cnt));",
    },
    InsnDoc {
        mnemonic: "rcl",
        summary: "带 CF 的循环左移",
        semantics: "rotate (dst, CF) left by cnt;",
    },
    InsnDoc {
        mnemonic: "rcr",
        summary: "带 CF 的循环右移",
        semantics: "rotate (dst, CF) right by cnt;",
    },
    // ---- 控制流 ----
    InsnDoc {
        mnemonic: "jmp",
        summary: "无条件跳转到目标",
        semantics: "ip = target; (cs = newseg if far)",
    },
    InsnDoc {
        mnemonic: "jcxz",
        summary: "cx==0 则跳转（不看 flags）",
        semantics: "if (cx == 0) ip = target;",
    },
    InsnDoc {
        mnemonic: "loop",
        summary: "cx-=1，cx!=0 则跳转",
        semantics: "cx--; if (cx != 0) ip = target;",
    },
    InsnDoc {
        mnemonic: "call",
        summary: "调用子程序：push 返回地址；跳转",
        semantics: "push ip; (push cs if far)\nip = target;",
    },
    InsnDoc {
        mnemonic: "ret",
        summary: "近返回：pop ip",
        semantics: "ip = pop16();",
    },
    InsnDoc {
        mnemonic: "retf",
        summary: "远返回：pop ip; pop cs",
        semantics: "ip = pop16(); cs = pop16();",
    },
    InsnDoc {
        mnemonic: "hlt",
        summary: "停机",
        semantics: "halted = true;",
    },
    InsnDoc {
        mnemonic: "nop",
        summary: "空操作（一个字节填充）",
        semantics: "/* no effect */",
    },
    // ---- 条件跳转族 ----
    InsnDoc {
        mnemonic: "je",
        summary: "ZF=1 则跳（=jz）",
        semantics: "if (ZF) ip = target;",
    },
    InsnDoc {
        mnemonic: "jz",
        summary: "ZF=1 则跳（=je）",
        semantics: "if (ZF) ip = target;",
    },
    InsnDoc {
        mnemonic: "jne",
        summary: "ZF=0 则跳（=jnz）",
        semantics: "if (!ZF) ip = target;",
    },
    InsnDoc {
        mnemonic: "jnz",
        summary: "ZF=0 则跳（=jne）",
        semantics: "if (!ZF) ip = target;",
    },
    InsnDoc {
        mnemonic: "jc",
        summary: "CF=1 则跳（=jb=jnae，无符号 <）",
        semantics: "if (CF) ip = target;",
    },
    InsnDoc {
        mnemonic: "jnc",
        summary: "CF=0 则跳（=jae=jnb，无符号 >=）",
        semantics: "if (!CF) ip = target;",
    },
    InsnDoc {
        mnemonic: "ja",
        summary: "CF=0 且 ZF=0 则跳（无符号 >）",
        semantics: "if (!CF && !ZF) ip = target;",
    },
    InsnDoc {
        mnemonic: "jbe",
        summary: "CF=1 或 ZF=1 则跳（无符号 <=）",
        semantics: "if (CF || ZF) ip = target;",
    },
    InsnDoc {
        mnemonic: "jl",
        summary: "SF!=OF 则跳（有符号 <）",
        semantics: "if (SF != OF) ip = target;",
    },
    InsnDoc {
        mnemonic: "jge",
        summary: "SF==OF 则跳（有符号 >=）",
        semantics: "if (SF == OF) ip = target;",
    },
    InsnDoc {
        mnemonic: "jg",
        summary: "ZF=0 且 SF==OF 则跳（有符号 >）",
        semantics: "if (!ZF && SF == OF) ip = target;",
    },
    InsnDoc {
        mnemonic: "jle",
        summary: "ZF=1 或 SF!=OF 则跳（有符号 <=）",
        semantics: "if (ZF || SF != OF) ip = target;",
    },
    InsnDoc {
        mnemonic: "js",
        summary: "SF=1 则跳（结果为负）",
        semantics: "if (SF) ip = target;",
    },
    InsnDoc {
        mnemonic: "jns",
        summary: "SF=0 则跳（结果非负）",
        semantics: "if (!SF) ip = target;",
    },
    InsnDoc {
        mnemonic: "jo",
        summary: "OF=1 则跳（有符号溢出）",
        semantics: "if (OF) ip = target;",
    },
    InsnDoc {
        mnemonic: "jno",
        summary: "OF=0 则跳（无符号溢出）",
        semantics: "if (!OF) ip = target;",
    },
    // ---- 中断 / I/O ----
    InsnDoc {
        mnemonic: "int",
        summary: "push flags; push cs; push ip; 走 IVT[n]",
        semantics: "push flags; push cs; push ip;\nip = mem16[n*4]; cs = mem16[n*4+2];",
    },
    InsnDoc {
        mnemonic: "iret",
        summary: "中断返回：pop ip; pop cs; pop flags",
        semantics: "ip = pop16(); cs = pop16(); flags = pop16();",
    },
    InsnDoc {
        mnemonic: "cli",
        summary: "关中断（IF=0）",
        semantics: "IF = 0;",
    },
    InsnDoc {
        mnemonic: "sti",
        summary: "开中断（IF=1）",
        semantics: "IF = 1;",
    },
    InsnDoc {
        mnemonic: "in",
        summary: "从 I/O 端口读字节到 al/ax",
        semantics: "al = port_in(port);",
    },
    InsnDoc {
        mnemonic: "out",
        summary: "把 al/ax 写到 I/O 端口",
        semantics: "port_out(port, al);",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_known_mnemonic() {
        let d = lookup("mov").expect("mov in table");
        assert_eq!(d.mnemonic, "mov");
        assert!(!d.summary.is_empty());
    }

    #[test]
    fn lookup_unknown_returns_none() {
        assert!(lookup("nonexistent").is_none());
    }

    #[test]
    fn table_covers_core_set() {
        // 抽查教材常见 30+ 条
        for m in [
            "mov", "push", "pop", "add", "sub", "cmp", "mul", "div", "and", "or", "xor", "shl",
            "shr", "jmp", "je", "jne", "loop", "call", "ret", "int", "iret", "hlt",
        ] {
            assert!(lookup(m).is_some(), "missing {m}");
        }
    }
}
