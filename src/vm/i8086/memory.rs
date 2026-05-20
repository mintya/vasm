use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MemError {
    #[error("memory access out of bounds at 0x{addr:05X} (size {size})")]
    OutOfBounds { addr: u32, size: usize },
}

#[derive(Debug, Clone)]
pub struct Memory {
    bytes: Vec<u8>,
}

impl Memory {
    pub fn new(size_kb: u32) -> Self {
        let bytes = vec![0u8; (size_kb as usize) * 1024];
        Self { bytes }
    }

    pub fn size(&self) -> u32 {
        self.bytes.len() as u32
    }

    /// 8086 物理地址：seg * 16 + off
    pub const fn phys(seg: u16, off: u16) -> u32 {
        ((seg as u32) << 4) + off as u32
    }

    pub fn read_u8(&self, addr: u32) -> Result<u8, MemError> {
        self.bytes
            .get(addr as usize)
            .copied()
            .ok_or(MemError::OutOfBounds { addr, size: 1 })
    }

    pub fn write_u8(&mut self, addr: u32, v: u8) -> Result<(), MemError> {
        let slot = self
            .bytes
            .get_mut(addr as usize)
            .ok_or(MemError::OutOfBounds { addr, size: 1 })?;
        *slot = v;
        Ok(())
    }

    pub fn read_u16(&self, addr: u32) -> Result<u16, MemError> {
        let lo = self.read_u8(addr)?;
        let hi = self
            .read_u8(
                addr.checked_add(1)
                    .ok_or(MemError::OutOfBounds { addr, size: 2 })?,
            )
            .map_err(|_| MemError::OutOfBounds { addr, size: 2 })?;
        Ok(u16::from_le_bytes([lo, hi]))
    }

    pub fn write_u16(&mut self, addr: u32, v: u16) -> Result<(), MemError> {
        let [lo, hi] = v.to_le_bytes();
        self.write_u8(addr, lo)?;
        self.write_u8(
            addr.checked_add(1)
                .ok_or(MemError::OutOfBounds { addr, size: 2 })?,
            hi,
        )
        .map_err(|_| MemError::OutOfBounds { addr, size: 2 })?;
        Ok(())
    }

    /// 批量写入（数据段加载用）。
    pub fn write_bytes(&mut self, addr: u32, data: &[u8]) -> Result<(), MemError> {
        let end = (addr as usize)
            .checked_add(data.len())
            .ok_or(MemError::OutOfBounds {
                addr,
                size: data.len(),
            })?;
        if end > self.bytes.len() {
            return Err(MemError::OutOfBounds {
                addr,
                size: data.len(),
            });
        }
        self.bytes[addr as usize..end].copy_from_slice(data);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phys_calculation() {
        assert_eq!(Memory::phys(0x1000, 0x0010), 0x10010);
        assert_eq!(Memory::phys(0xFFFF, 0xFFFF), 0xFFFF * 16 + 0xFFFF);
    }

    #[test]
    fn read_write_u8() {
        let mut mem = Memory::new(1);
        mem.write_u8(0x100, 0xAB).unwrap();
        assert_eq!(mem.read_u8(0x100).unwrap(), 0xAB);
    }

    #[test]
    fn read_write_u16_little_endian() {
        let mut mem = Memory::new(1);
        mem.write_u16(0x200, 0x1234).unwrap();
        assert_eq!(mem.read_u8(0x200).unwrap(), 0x34);
        assert_eq!(mem.read_u8(0x201).unwrap(), 0x12);
        assert_eq!(mem.read_u16(0x200).unwrap(), 0x1234);
    }

    #[test]
    fn out_of_bounds_byte() {
        let mem = Memory::new(1);
        let err = mem.read_u8(2000).unwrap_err();
        assert!(matches!(
            err,
            MemError::OutOfBounds {
                addr: 2000,
                size: 1
            }
        ));
    }

    #[test]
    fn out_of_bounds_word_straddle() {
        let mut mem = Memory::new(1);
        let last = mem.size() - 1;
        let err = mem.write_u16(last, 0).unwrap_err();
        assert!(matches!(err, MemError::OutOfBounds { size: 2, .. }));
    }

    #[test]
    fn write_bytes_bulk() {
        let mut mem = Memory::new(1);
        mem.write_bytes(0x10, &[1, 2, 3, 4]).unwrap();
        assert_eq!(mem.read_u8(0x10).unwrap(), 1);
        assert_eq!(mem.read_u8(0x13).unwrap(), 4);
    }
}
