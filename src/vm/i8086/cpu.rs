#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reg16 {
    Ax,
    Bx,
    Cx,
    Dx,
    Si,
    Di,
    Bp,
    Sp,
    Cs,
    Ds,
    Ss,
    Es,
    Ip,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reg8 {
    Al,
    Ah,
    Bl,
    Bh,
    Cl,
    Ch,
    Dl,
    Dh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegRef {
    R16(Reg16),
    R8(Reg8),
}

impl RegRef {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "ax" => Some(Self::R16(Reg16::Ax)),
            "bx" => Some(Self::R16(Reg16::Bx)),
            "cx" => Some(Self::R16(Reg16::Cx)),
            "dx" => Some(Self::R16(Reg16::Dx)),
            "si" => Some(Self::R16(Reg16::Si)),
            "di" => Some(Self::R16(Reg16::Di)),
            "bp" => Some(Self::R16(Reg16::Bp)),
            "sp" => Some(Self::R16(Reg16::Sp)),
            "cs" => Some(Self::R16(Reg16::Cs)),
            "ds" => Some(Self::R16(Reg16::Ds)),
            "ss" => Some(Self::R16(Reg16::Ss)),
            "es" => Some(Self::R16(Reg16::Es)),
            "ip" => Some(Self::R16(Reg16::Ip)),
            "al" => Some(Self::R8(Reg8::Al)),
            "ah" => Some(Self::R8(Reg8::Ah)),
            "bl" => Some(Self::R8(Reg8::Bl)),
            "bh" => Some(Self::R8(Reg8::Bh)),
            "cl" => Some(Self::R8(Reg8::Cl)),
            "ch" => Some(Self::R8(Reg8::Ch)),
            "dl" => Some(Self::R8(Reg8::Dl)),
            "dh" => Some(Self::R8(Reg8::Dh)),
            _ => None,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Flags {
    pub cf: bool,
    pub pf: bool,
    pub af: bool,
    pub zf: bool,
    pub sf: bool,
    pub tf: bool,
    pub if_: bool,
    pub df: bool,
    pub of: bool,
}

impl Flags {
    /// 打包成 8086 FLAGS 寄存器格式（含恒为 1 的 bit1）。
    pub fn to_u16(self) -> u16 {
        let mut v: u16 = 0x0002; // bit 1 reserved, always 1 on 8086
        if self.cf {
            v |= 1 << 0;
        }
        if self.pf {
            v |= 1 << 2;
        }
        if self.af {
            v |= 1 << 4;
        }
        if self.zf {
            v |= 1 << 6;
        }
        if self.sf {
            v |= 1 << 7;
        }
        if self.tf {
            v |= 1 << 8;
        }
        if self.if_ {
            v |= 1 << 9;
        }
        if self.df {
            v |= 1 << 10;
        }
        if self.of {
            v |= 1 << 11;
        }
        v
    }

    pub fn from_u16(v: u16) -> Self {
        Self {
            cf: v & (1 << 0) != 0,
            pf: v & (1 << 2) != 0,
            af: v & (1 << 4) != 0,
            zf: v & (1 << 6) != 0,
            sf: v & (1 << 7) != 0,
            tf: v & (1 << 8) != 0,
            if_: v & (1 << 9) != 0,
            df: v & (1 << 10) != 0,
            of: v & (1 << 11) != 0,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Cpu {
    pub ax: u16,
    pub bx: u16,
    pub cx: u16,
    pub dx: u16,
    pub si: u16,
    pub di: u16,
    pub bp: u16,
    pub sp: u16,
    pub cs: u16,
    pub ds: u16,
    pub ss: u16,
    pub es: u16,
    pub ip: u16,
    pub flags: Flags,
}

impl Cpu {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn r16(&self, r: Reg16) -> u16 {
        match r {
            Reg16::Ax => self.ax,
            Reg16::Bx => self.bx,
            Reg16::Cx => self.cx,
            Reg16::Dx => self.dx,
            Reg16::Si => self.si,
            Reg16::Di => self.di,
            Reg16::Bp => self.bp,
            Reg16::Sp => self.sp,
            Reg16::Cs => self.cs,
            Reg16::Ds => self.ds,
            Reg16::Ss => self.ss,
            Reg16::Es => self.es,
            Reg16::Ip => self.ip,
        }
    }

    pub fn set_r16(&mut self, r: Reg16, v: u16) {
        match r {
            Reg16::Ax => self.ax = v,
            Reg16::Bx => self.bx = v,
            Reg16::Cx => self.cx = v,
            Reg16::Dx => self.dx = v,
            Reg16::Si => self.si = v,
            Reg16::Di => self.di = v,
            Reg16::Bp => self.bp = v,
            Reg16::Sp => self.sp = v,
            Reg16::Cs => self.cs = v,
            Reg16::Ds => self.ds = v,
            Reg16::Ss => self.ss = v,
            Reg16::Es => self.es = v,
            Reg16::Ip => self.ip = v,
        }
    }

    pub fn r8(&self, r: Reg8) -> u8 {
        let (parent, high) = parent_and_high(r);
        let word = self.r16(parent);
        if high { (word >> 8) as u8 } else { word as u8 }
    }

    pub fn set_r8(&mut self, r: Reg8, v: u8) {
        let (parent, high) = parent_and_high(r);
        let word = self.r16(parent);
        let new = if high {
            (word & 0x00FF) | ((v as u16) << 8)
        } else {
            (word & 0xFF00) | (v as u16)
        };
        self.set_r16(parent, new);
    }
}

fn parent_and_high(r: Reg8) -> (Reg16, bool) {
    match r {
        Reg8::Al => (Reg16::Ax, false),
        Reg8::Ah => (Reg16::Ax, true),
        Reg8::Bl => (Reg16::Bx, false),
        Reg8::Bh => (Reg16::Bx, true),
        Reg8::Cl => (Reg16::Cx, false),
        Reg8::Ch => (Reg16::Cx, true),
        Reg8::Dl => (Reg16::Dx, false),
        Reg8::Dh => (Reg16::Dx, true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r16_round_trip() {
        let mut cpu = Cpu::new();
        cpu.set_r16(Reg16::Ax, 0x1234);
        assert_eq!(cpu.r16(Reg16::Ax), 0x1234);
        assert_eq!(cpu.ax, 0x1234);
    }

    #[test]
    fn r8_aliases_share_with_parent() {
        let mut cpu = Cpu::new();
        cpu.set_r16(Reg16::Ax, 0x1234);
        assert_eq!(cpu.r8(Reg8::Ah), 0x12);
        assert_eq!(cpu.r8(Reg8::Al), 0x34);

        cpu.set_r8(Reg8::Al, 0xFF);
        assert_eq!(cpu.ax, 0x12FF);
        cpu.set_r8(Reg8::Ah, 0xAB);
        assert_eq!(cpu.ax, 0xABFF);
    }

    #[test]
    fn from_name_round_trip() {
        assert_eq!(RegRef::from_name("ax"), Some(RegRef::R16(Reg16::Ax)));
        assert_eq!(RegRef::from_name("ah"), Some(RegRef::R8(Reg8::Ah)));
        assert_eq!(RegRef::from_name("dl"), Some(RegRef::R8(Reg8::Dl)));
        assert_eq!(RegRef::from_name("ip"), Some(RegRef::R16(Reg16::Ip)));
        assert_eq!(RegRef::from_name("foo"), None);
    }

    #[test]
    fn flags_round_trip_via_u16() {
        let f = Flags {
            cf: true,
            zf: true,
            of: true,
            ..Flags::default()
        };
        let packed = f.to_u16();
        let f2 = Flags::from_u16(packed);
        assert_eq!(f, f2);
    }

    #[test]
    fn flags_reserved_bit1_is_one() {
        let f = Flags::default();
        assert_eq!(f.to_u16() & 0x0002, 0x0002);
    }
}
