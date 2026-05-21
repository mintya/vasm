use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MemError {
    #[error("memory access out of bounds at 0x{addr:05X} (size {size})")]
    OutOfBounds { addr: u32, size: usize },
}

#[derive(Debug, Clone)]
pub struct Memory {
    bytes: Vec<u8>,
    /// undo 用：开启后，write_u8 会把每个被覆盖位置的 (addr, 旧值) 追加到这里。
    /// `Vm::step_with_snapshot` 在 step 前 `start_recording()`，结束后
    /// `take_recording()` 收集进 Snapshot。
    recording: Option<Vec<(u32, u8)>>,
}

impl Memory {
    pub fn new(size_kb: u32) -> Self {
        let bytes = vec![0u8; (size_kb as usize) * 1024];
        Self {
            bytes,
            recording: None,
        }
    }

    pub fn size(&self) -> u32 {
        self.bytes.len() as u32
    }

    /// 开始记录所有 write_u8 的 (addr, 旧值)。
    pub fn start_recording(&mut self) {
        self.recording = Some(Vec::new());
    }

    /// 取走累计的写入记录并停止记录。
    pub fn take_recording(&mut self) -> Vec<(u32, u8)> {
        self.recording.take().unwrap_or_default()
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
        let old = *slot;
        *slot = v;
        if let Some(rec) = self.recording.as_mut() {
            rec.push((addr, old));
        }
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

    /// 批量写入（数据段加载用）。注意：**不走 recording**——仅供 boot 时
    /// 加载数据段使用，不在 step 期间调用。
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

    #[test]
    fn write_recording_captures_old_values() {
        let mut mem = Memory::new(1);
        mem.write_u8(0x10, 0xAA).unwrap();
        // 启动录制 → 后续 write 应捕获 (addr, old)
        mem.start_recording();
        mem.write_u8(0x10, 0xBB).unwrap();
        mem.write_u8(0x11, 0xCC).unwrap();
        let rec = mem.take_recording();
        assert_eq!(rec, vec![(0x10, 0xAA), (0x11, 0x00)]);
        // 停止后再写不再记录
        mem.write_u8(0x12, 0xDD).unwrap();
        assert!(mem.take_recording().is_empty());
    }
}
