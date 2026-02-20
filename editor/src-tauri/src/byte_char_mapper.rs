/// 将 UTF-8 byte offset 与“字符索引”互转的映射器。
///
/// 这里的“字符”定义为：
/// - 一个 Unicode 标量值（Rust 的 `char`，也就是 `str.chars()` 的单位）
/// - 但换行序列 `\r\n` 视作一个字符（即 CRLF 是一个单位）
/// - 单独的 `\n` 也是一个字符
///
/// 这在后端（lexer/span 以 byte offset 表达）与前端（需要按字符单位展示/定位）之间很常见。
///
/// 映射表语义：
/// - `byte_to_char[byte]`：byte 边界 `byte` 对应的字符边界索引（0..=char_len）
///   - 如果 byte 落在多字节字符内部，映射到该字符的起始字符索引
///   - 如果 byte 正好是字符边界，则返回“该边界之前的字符数”
/// - `char_to_byte[ch]`：第 `ch` 个字符边界对应的 byte offset（0..=byte_len）
#[derive(Debug, Clone)]
pub struct ByteCharMapper {
    byte_to_char: Vec<u32>,
    char_to_byte: Vec<u32>,
}

impl ByteCharMapper {
    /// 为给定源码构建映射表。
    pub fn new(source: &str) -> Self {
        let byte_len = source.len();
        let mut byte_to_char = vec![0u32; byte_len + 1];

        // 先粗略预估容量：至少与 `chars()` 数量相当
        let mut char_to_byte: Vec<u32> = Vec::with_capacity(source.chars().count() + 1);

        // 空串特判
        if byte_len == 0 {
            char_to_byte.push(0);
            return Self {
                byte_to_char,
                char_to_byte,
            };
        }

        let mut char_index = 0u32;
        let mut iter = source.char_indices().peekable();

        while let Some((start, ch)) = iter.next() {
            // 记录该字符边界
            char_to_byte.push(start as u32);

            // 计算“这个字符单位”占用的 byte 数
            let unit_len = if ch == '\r' {
                // CRLF 视作一个字符
                if let Some(&(next_start, next_ch)) = iter.peek() {
                    if next_ch == '\n' && next_start == start + 1 {
                        // 消费掉 '\n'
                        let _ = iter.next();
                        2
                    } else {
                        1
                    }
                } else {
                    1
                }
            } else if ch == '\n' {
                1
            } else {
                ch.len_utf8()
            };

            // 将该字符单位覆盖的 byte 边界都映射到 char_index
            // - start 是该字符起始 byte
            // - start+unit_len 是该字符结束边界（不在这里写，交给下一个字符或末尾写入 char_index+1）
            byte_to_char[start] = char_index;
            for b in (start + 1)..(start + unit_len) {
                if b <= byte_len {
                    byte_to_char[b] = char_index;
                }
            }

            char_index += 1;
        }

        // 末尾字符边界
        char_to_byte.push(byte_len as u32);
        byte_to_char[byte_len] = char_index;

        Self {
            byte_to_char,
            char_to_byte,
        }
    }

    /// 文档总字节长度。
    pub fn byte_len(&self) -> u32 {
        self.byte_to_char.len().saturating_sub(1) as u32
    }

    /// 字符单位总数。
    pub fn char_len(&self) -> u32 {
        self.char_to_byte.len().saturating_sub(1) as u32
    }

    /// byte offset -> char index。
    pub fn byte_to_char(&self, byte: u32) -> u32 {
        let b = byte.min(self.byte_len());
        self.byte_to_char[b as usize]
    }

    /// char index -> byte offset。
    pub fn char_to_byte(&self, ch: u32) -> u32 {
        let c = ch.min(self.char_len());
        self.char_to_byte[c as usize]
    }

    /// 将 byte 区间转成 char 区间（按边界转换，保证 from<=to）。
    pub fn byte_range_to_char(&self, from: u32, to: u32) -> (u32, u32) {
        let a = self.byte_to_char(from);
        let b = self.byte_to_char(to);
        if a <= b {
            (a, b)
        } else {
            (b, a)
        }
    }

    /// 将 char 区间转成 byte 区间（按边界转换，保证 from<=to）。
    pub fn char_range_to_byte(&self, from: u32, to: u32) -> (u32, u32) {
        let a = self.char_to_byte(from);
        let b = self.char_to_byte(to);
        if a <= b {
            (a, b)
        } else {
            (b, a)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_ascii_and_newlines() {
        let s = "a\n\r\nb";
        let m = ByteCharMapper::new(s);
        // 'a', '\n', '\r\n', 'b' => 4
        assert_eq!(m.char_len(), 4);
        assert_eq!(m.byte_len(), s.len() as u32);
        assert_eq!(m.char_to_byte(0), 0);
        assert_eq!(m.char_to_byte(1), 1);
        assert_eq!(m.char_to_byte(2), 2);
        assert_eq!(m.char_to_byte(3), 4);
        assert_eq!(m.char_to_byte(4), 5);

        // byte->char: boundaries 0..=5
        assert_eq!(m.byte_to_char(0), 0);
        assert_eq!(m.byte_to_char(1), 1);
        assert_eq!(m.byte_to_char(2), 2);
        assert_eq!(m.byte_to_char(3), 2); // inside CRLF maps to same char index
        assert_eq!(m.byte_to_char(4), 3);
        assert_eq!(m.byte_to_char(5), 4);
    }

    #[test]
    fn maps_unicode_scalar_as_one_char_unit() {
        let s = "a😊\n\r\nb";
        // bytes: a(1) + 😊(4) + \n(1) + \r\n(2) + b(1) = 9
        assert_eq!(s.len() as u32, 9);
        let m = ByteCharMapper::new(s);
        // 'a', '😊', '\n', '\r\n', 'b' => 5
        assert_eq!(m.char_len(), 5);
        assert_eq!(m.char_to_byte(0), 0);
        assert_eq!(m.char_to_byte(1), 1);
        assert_eq!(m.char_to_byte(2), 5);
        assert_eq!(m.char_to_byte(3), 6);
        assert_eq!(m.char_to_byte(4), 8);
        assert_eq!(m.char_to_byte(5), 9);

        // inside the emoji multi-byte sequence: map back to char index 1
        assert_eq!(m.byte_to_char(2), 1);
        assert_eq!(m.byte_to_char(3), 1);
        assert_eq!(m.byte_to_char(4), 1);
    }
}
