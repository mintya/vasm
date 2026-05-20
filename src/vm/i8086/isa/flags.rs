use crate::vm::i8086::cpu::Flags;

pub fn parity(byte: u8) -> bool {
    (byte.count_ones() & 1) == 0
}

pub fn after_add_u16(a: u16, b: u16, result: u16, flags: &mut Flags) {
    flags.cf = result < a;
    flags.of = ((a ^ result) & (b ^ result)) & 0x8000 != 0;
    flags.af = ((a ^ b ^ result) & 0x10) != 0;
    flags.zf = result == 0;
    flags.sf = result & 0x8000 != 0;
    flags.pf = parity(result as u8);
}

pub fn after_sub_u16(a: u16, b: u16, result: u16, flags: &mut Flags) {
    flags.cf = b > a;
    flags.of = ((a ^ b) & (a ^ result)) & 0x8000 != 0;
    flags.af = ((a ^ b ^ result) & 0x10) != 0;
    flags.zf = result == 0;
    flags.sf = result & 0x8000 != 0;
    flags.pf = parity(result as u8);
}

pub fn after_add_u8(a: u8, b: u8, result: u8, flags: &mut Flags) {
    flags.cf = result < a;
    flags.of = ((a ^ result) & (b ^ result)) & 0x80 != 0;
    flags.af = ((a ^ b ^ result) & 0x10) != 0;
    flags.zf = result == 0;
    flags.sf = result & 0x80 != 0;
    flags.pf = parity(result);
}

pub fn after_sub_u8(a: u8, b: u8, result: u8, flags: &mut Flags) {
    flags.cf = b > a;
    flags.of = ((a ^ b) & (a ^ result)) & 0x80 != 0;
    flags.af = ((a ^ b ^ result) & 0x10) != 0;
    flags.zf = result == 0;
    flags.sf = result & 0x80 != 0;
    flags.pf = parity(result);
}

/// inc/dec：与 add/sub 类似，但不动 CF。
pub fn after_inc_u16(a: u16, result: u16, flags: &mut Flags) {
    let cf = flags.cf;
    after_add_u16(a, 1, result, flags);
    flags.cf = cf;
}

pub fn after_dec_u16(a: u16, result: u16, flags: &mut Flags) {
    let cf = flags.cf;
    after_sub_u16(a, 1, result, flags);
    flags.cf = cf;
}

pub fn after_inc_u8(a: u8, result: u8, flags: &mut Flags) {
    let cf = flags.cf;
    after_add_u8(a, 1, result, flags);
    flags.cf = cf;
}

pub fn after_dec_u8(a: u8, result: u8, flags: &mut Flags) {
    let cf = flags.cf;
    after_sub_u8(a, 1, result, flags);
    flags.cf = cf;
}

/// and/or/xor/test: CF=OF=0, AF undefined (我们不动), 按 result 更新 ZF/SF/PF。
pub fn after_logical_u16(result: u16, flags: &mut Flags) {
    flags.cf = false;
    flags.of = false;
    flags.zf = result == 0;
    flags.sf = result & 0x8000 != 0;
    flags.pf = parity(result as u8);
}

pub fn after_logical_u8(result: u8, flags: &mut Flags) {
    flags.cf = false;
    flags.of = false;
    flags.zf = result == 0;
    flags.sf = result & 0x80 != 0;
    flags.pf = parity(result);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parity_even_count() {
        assert!(parity(0b0000_0000));
        assert!(parity(0b0000_0011));
        assert!(!parity(0b0000_0001));
        assert!(parity(0b1111_1111));
    }

    #[test]
    fn add_sets_zf_when_zero() {
        let mut f = Flags::default();
        after_add_u16(0xFFFF, 0x0001, 0x0000, &mut f);
        assert!(f.zf);
        assert!(f.cf);
        assert!(!f.of);
        assert!(!f.sf);
    }

    #[test]
    fn add_signed_overflow_positive_to_negative() {
        // 0x7FFF + 1 = 0x8000 → 正溢出
        let mut f = Flags::default();
        after_add_u16(0x7FFF, 0x0001, 0x8000, &mut f);
        assert!(f.of);
        assert!(f.sf);
        assert!(!f.cf);
    }

    #[test]
    fn sub_borrow_sets_cf() {
        let mut f = Flags::default();
        after_sub_u16(0x0001, 0x0002, 0xFFFF, &mut f);
        assert!(f.cf);
        assert!(f.sf);
    }

    #[test]
    fn sub_signed_overflow_negative_to_positive() {
        // 0x8000 - 1 = 0x7FFF → 负溢出（OF）
        let mut f = Flags::default();
        after_sub_u16(0x8000, 0x0001, 0x7FFF, &mut f);
        assert!(f.of);
        assert!(!f.sf);
    }

    #[test]
    fn inc_does_not_touch_cf() {
        let mut f = Flags {
            cf: true,
            ..Flags::default()
        };
        after_inc_u16(0x0000, 0x0001, &mut f);
        assert!(f.cf, "inc should preserve cf");
    }

    #[test]
    fn af_set_when_bit3_carry() {
        let mut f = Flags::default();
        after_add_u8(0x0F, 0x01, 0x10, &mut f);
        assert!(f.af);
    }
}
