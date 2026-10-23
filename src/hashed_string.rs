#[repr(C)]
pub struct HashedString {
    pub length: u32,
    pub hash: u32,
    pub string: [u8; 64],
}

impl HashedString {
    pub fn set(&mut self, replacement: &str) {
        // Keep room for trailing NUL — oversize writes panic'd standalone css_first.
        let max = self.string.len().saturating_sub(1);
        let bytes = replacement.as_bytes();
        let n = bytes.len().min(max);
        self.length = n as u32;
        self.string[..n].copy_from_slice(&bytes[..n]);
        self.string[n] = 0;
        if n < self.string.len() - 1 {
            // clear a bit of leftover junk for cleaner logs
            for b in &mut self.string[n + 1..] {
                *b = 0;
            }
        }
    }

    pub fn as_str(&self) -> &str {
        let len = self
            .string
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(self.string.len())
            .min(self.string.len());
        std::str::from_utf8(&self.string[..len]).unwrap_or("")
    }
}
