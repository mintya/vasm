use std::collections::VecDeque;

/// Console I/O 缓冲，归 Vm 所有（M5）。
///
/// - `output`：DOS/BIOS stub 写入；UI 渲染层按 encoding 解码后显示。
/// - `input`：UI keymap 在 Console 焦点下入队；stub 弹出供程序读取。
/// - `waiting_for_input`：step 协议位——stub 在缓冲为空时设为 true，
///   `Vm::step` 把 ip 退回这条 int 指令并返回 `StepOutcome::WaitingForInput`，
///   让 App 切焦点等用户敲字符；下次 step 时 stub 重新尝试。
#[derive(Debug, Default, Clone)]
pub struct ConsoleIo {
    output: Vec<u8>,
    input: VecDeque<u8>,
    waiting_for_input: bool,
    interrupts: u64,
    display_mode: u8,
    cursor: (u8, u8),
}

impl ConsoleIo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_output(&mut self, byte: u8) {
        self.output.push(byte);
    }

    pub fn push_output_bytes(&mut self, bytes: &[u8]) {
        self.output.extend_from_slice(bytes);
    }

    pub fn output(&self) -> &[u8] {
        &self.output
    }

    pub fn clear_output(&mut self) {
        self.output.clear();
    }

    pub fn push_input(&mut self, byte: u8) {
        self.input.push_back(byte);
    }

    pub fn pop_input(&mut self) -> Option<u8> {
        self.input.pop_front()
    }

    pub fn peek_input(&self) -> Option<u8> {
        self.input.front().copied()
    }

    pub fn has_input(&self) -> bool {
        !self.input.is_empty()
    }

    pub fn input_len(&self) -> usize {
        self.input.len()
    }

    /// 当前队列里还未被消费的输入字节快照。UI 渲染层用它做"预回显"。
    pub fn input_bytes(&self) -> Vec<u8> {
        self.input.iter().copied().collect()
    }

    /// 从队尾弹出一个字节。Backspace 视觉删字符时用。
    pub fn pop_input_back(&mut self) -> Option<u8> {
        self.input.pop_back()
    }

    pub fn clear_input(&mut self) {
        self.input.clear();
    }

    pub fn waiting_for_input(&self) -> bool {
        self.waiting_for_input
    }

    pub fn set_waiting(&mut self, v: bool) {
        self.waiting_for_input = v;
    }

    pub fn bump_interrupts(&mut self) {
        self.interrupts += 1;
    }

    pub fn interrupts(&self) -> u64 {
        self.interrupts
    }

    pub fn display_mode(&self) -> u8 {
        self.display_mode
    }

    pub fn set_display_mode(&mut self, mode: u8) {
        self.display_mode = mode;
    }

    pub fn cursor(&self) -> (u8, u8) {
        self.cursor
    }

    pub fn set_cursor(&mut self, row: u8, col: u8) {
        self.cursor = (row, col);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_push_and_read() {
        let mut io = ConsoleIo::new();
        io.push_output(b'H');
        io.push_output_bytes(b"ello");
        assert_eq!(io.output(), b"Hello");
    }

    #[test]
    fn input_fifo_order() {
        let mut io = ConsoleIo::new();
        io.push_input(b'a');
        io.push_input(b'b');
        assert_eq!(io.peek_input(), Some(b'a'));
        assert_eq!(io.pop_input(), Some(b'a'));
        assert_eq!(io.pop_input(), Some(b'b'));
        assert_eq!(io.pop_input(), None);
    }

    #[test]
    fn waiting_flag_round_trip() {
        let mut io = ConsoleIo::new();
        assert!(!io.waiting_for_input());
        io.set_waiting(true);
        assert!(io.waiting_for_input());
        io.set_waiting(false);
        assert!(!io.waiting_for_input());
    }

    #[test]
    fn input_bytes_snapshot_and_pop_back() {
        let mut io = ConsoleIo::new();
        io.push_input(b'a');
        io.push_input(b'b');
        io.push_input(b'c');
        assert_eq!(io.input_bytes(), vec![b'a', b'b', b'c']);
        assert_eq!(io.pop_input_back(), Some(b'c'));
        assert_eq!(io.input_bytes(), vec![b'a', b'b']);
        // pop_back 不影响 FIFO 头
        assert_eq!(io.pop_input(), Some(b'a'));
    }
}
