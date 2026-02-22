use std::vec;

use logos::{Lexer, Logos};

/// 词法与语法种类枚举。
///
/// 该枚举既包含**词法 Token**（比如 `Comma`、`Identifier`），也包含**语法树节点**（比如 `NODE_ROOT`）。
///
/// # 示例
/// ```rust
/// use symi::rowan::lexer::SyntaxKind;
///
/// // 判断是否是“琐碎”文本（空白/注释）。
/// assert!(SyntaxKind::Whitespace.is_trivia());
/// assert!(SyntaxKind::Comment.is_trivia());
/// ```

#[derive(
    Logos,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    strum::EnumIs,
    strum::IntoStaticStr,
    strum::EnumTryAs,
)]
#[repr(u16)]
#[allow(non_camel_case_types, dead_code)]
pub enum SyntaxKind {
    /// Whitespace (spaces and tabs, not newlines)
    #[regex("[ \t]+")]
    Whitespace,
    /// Newline
    #[regex("\r?\n")]
    Newline,
    /// Comment (from '//' to end of line), including the ending line break
    #[regex("//[^\r\n]*", allow_greedy = true)]
    Comment,
    /// Comma ','
    #[token(",")]
    Comma,
    /// Colon ':'
    #[token(":")]
    Colon,
    /// Semicolon ';'
    #[token(";")]
    Semicolon,
    /// At '@'
    /// Used for pitch chain connector
    #[token("@")]
    At,
    /// ParenthesisPair '()'
    #[token("()")]
    ParenthesisPair,
    /// PitchSpellOctave (e.g. C#4, Db3, A5, Gb6)
    /// Octave is -9 to 19
    #[regex(r"[A-G](#|b)*(-[1-9]|1?[0-9])")]
    PitchSpellOctave,
    /// PitchSpellSimple
    /// Octave is omitted or +/-
    #[regex(r"[A-G](#|b)*[+-]*")]
    PitchSpellSimple,
    /// PitchFrequency in Hz (e.g. 440.0, 261.63)
    /// Must be greater than 1, less than 1e8
    /// Allows negative and zero for edo grammar sugar.
    #[regex(r"-?\d+(\.\d+)?", |lex| lex.slice().parse::<f32>().ok().filter(|&f| f.abs() < 1e8).is_some())]
    PitchFrequency,
    /// PitchRatio (e.g. 3/2, 5/4)
    /// Numerator and denominator are positive integers (u16) and >0
    /// !!Also used for TimeSignature denominators!!
    #[regex(r"\d+/\d+", |lex|check_u16_groups(lex,"","/",2..3))]
    PitchRatio,
    /// PitchEdo (e.g. 7\12, 5\19)
    /// Step is integer (i16), Divisions is positive integer (u16) and >0
    #[regex(r"-?\d+\\\d+", |lex|check_edo_groups(lex,"","\\",2..3))]
    PitchEdo,
    /// PitchCents (e.g. 100c, -50c)
    /// Cents value is a signed integer (i32)
    #[regex(r"-?\d+c", |lex| lex.slice()[..lex.slice().len()-1].parse::<i32>().is_ok())]
    PitchCents,
    /// PitchRest
    #[regex(r"\.+", priority = 1)]
    PitchRest,
    /// PitchSustain
    #[token("-", priority = 1)]
    PitchSustain,
    /// Identifier (macro names, etc.)
    #[regex(r"[A-Za-z_][A-Za-z0-9_]*", priority = 0)]
    Identifier,
    /// DurationCommas
    #[regex(r"\[,+\]")]
    DurationCommas,
    /// DurationFraction
    #[regex(r"\[-?\d+(:\d+)?\]", |lex|check_u16_groups(lex,"[-]",":",1..3))]
    DurationFraction,
    /// Quantize
    #[regex(r"\{\d+(:\d+)?\}", |lex|check_u16_groups(lex,"{}",":",1..3))]
    Quantize,

    // ==== Other Tokens ====
    /// Equals '='
    /// Used for BPM & BasePitch definitions
    #[token("=")]
    Equals,
    /// LAngle '<'
    /// Used for base pitch definitions
    #[token("<")]
    LAngle,
    /// RAngle '>'
    /// Used for base pitch definitions
    #[token(">")]
    RAngle,
    /// LParen '('
    /// Used for macro invocations and BPM / TimeSignature changes
    #[token("(")]
    LParen,
    /// RParen ')'
    /// Used for macro invocations and BPM / TimeSignature changes
    #[token(")")]
    RParen,

    Error,

    // ==== Rowan Nodes ====
    NODE_ROOT,
    NODE_MACRODEF_SIMPLE,
    NODE_MACRODEF_RELATIVE,
    NODE_MACRODEF_COMPLEX,
    NODE_MACRODEF_COMPLEX_BODY,
    NODE_GHOST_LINE,
    NODE_NORMAL_LINE,
    NODE_NOTE_GROUP,
    NODE_NOTE,
    NODE_MACRO_INVOKE,
    NODE_BASE_PITCH_DEF,
    NODE_BPM_DEF,
    NODE_TIME_SIGNATURE_DEF,
}

/// 检查分隔后的各段是否能解析为正的 `u16`。
///
/// 该函数用于 logos 的回调验证，主要约束输入格式。
///
fn check_u16_groups(
    lex: &Lexer<SyntaxKind>,
    ignore_chars: &str,
    separators: &str,
    allow_parts_count_range: std::ops::Range<usize>,
) -> bool {
    let s = lex.slice();
    let trimmed: String = s.chars().filter(|c| !ignore_chars.contains(*c)).collect();
    let parts = separators.chars().fold(vec![trimmed.as_str()], |acc, sep| {
        acc.into_iter()
            .flat_map(|part| part.split(sep))
            .collect::<Vec<&str>>()
    });
    parts.len() >= allow_parts_count_range.start
        && parts.len() < allow_parts_count_range.end
        && parts
            .iter()
            .all(|part| part.parse::<u16>().map(|v| v > 0).unwrap_or(false))
}

fn check_edo_groups(
    lex: &Lexer<SyntaxKind>,
    ignore_chars: &str,
    separators: &str,
    allow_parts_count_range: std::ops::Range<usize>,
) -> bool {
    let s = lex.slice();
    let trimmed: String = s.chars().filter(|c| !ignore_chars.contains(*c)).collect();
    let parts = separators.chars().fold(vec![trimmed.as_str()], |acc, sep| {
        acc.into_iter()
            .flat_map(|part| part.split(sep))
            .collect::<Vec<&str>>()
    });
    parts.len() >= allow_parts_count_range.start
        && parts.len() < allow_parts_count_range.end
        && parts.iter().enumerate().all(|(i, part)| {
            if i == 0 {
                part.parse::<i16>().is_ok()
            } else {
                part.parse::<u16>().map(|v| v > 0).unwrap_or(false)
            }
        })
}

impl From<SyntaxKind> for rowan::SyntaxKind {
    /// 将自定义的 `SyntaxKind` 转为 rowan 可识别的 `rowan::SyntaxKind`。
    ///
    /// # 示例
    /// ```rust
    /// use symi::rowan::lexer::SyntaxKind;
    ///
    /// let raw: rowan::SyntaxKind = SyntaxKind::Comma.into();
    /// let back: SyntaxKind = raw.into();
    /// assert_eq!(back, SyntaxKind::Comma);
    /// ```
    fn from(kind: SyntaxKind) -> Self {
        rowan::SyntaxKind(kind as u16)
    }
}

impl From<rowan::SyntaxKind> for SyntaxKind {
    /// 将 rowan 的 `rowan::SyntaxKind` 转回本工程的 `SyntaxKind`。
    ///
    /// # 示例
    /// ```rust
    /// use symi::rowan::lexer::SyntaxKind;
    ///
    /// let raw: rowan::SyntaxKind = SyntaxKind::Semicolon.into();
    /// let kind: SyntaxKind = raw.into();
    /// assert_eq!(kind, SyntaxKind::Semicolon);
    /// ```
    fn from(raw: rowan::SyntaxKind) -> Self {
        // Safety: transmute u16 back to enum
        unsafe { std::mem::transmute(raw.0) }
    }
}

impl SyntaxKind {
    /// 判断该种类是否属于“琐碎”文本（空白/换行/注释）。
    ///
    /// 解析时，这些 Token 不计入“语义游标”，但仍会作为事件写入语法树，
    /// 以保持源代码位置信息和格式。
    ///
    /// # 示例
    /// ```rust
    /// use symi::rowan::lexer::SyntaxKind;
    ///
    /// assert!(SyntaxKind::Whitespace.is_trivia());
    /// assert!(!SyntaxKind::Identifier.is_trivia());
    /// ```
    pub fn is_trivia(&self) -> bool {
        matches!(self, SyntaxKind::Whitespace | SyntaxKind::Comment)
    }

    pub fn is_token(&self) -> bool {
        !self.is_node()
    }
    pub fn is_node(&self) -> bool {
        match self {
            SyntaxKind::NODE_ROOT
            | SyntaxKind::NODE_MACRODEF_SIMPLE
            | SyntaxKind::NODE_MACRODEF_RELATIVE
            | SyntaxKind::NODE_MACRODEF_COMPLEX
            | SyntaxKind::NODE_MACRODEF_COMPLEX_BODY
            | SyntaxKind::NODE_GHOST_LINE
            | SyntaxKind::NODE_NORMAL_LINE
            | SyntaxKind::NODE_NOTE_GROUP
            | SyntaxKind::NODE_NOTE
            | SyntaxKind::NODE_MACRO_INVOKE
            | SyntaxKind::NODE_BASE_PITCH_DEF
            | SyntaxKind::NODE_BPM_DEF
            | SyntaxKind::NODE_TIME_SIGNATURE_DEF => true,
            _ => false,
        }
    }

    pub fn is_pitch(&self) -> bool {
        matches!(
            self,
            SyntaxKind::PitchSpellOctave
                | SyntaxKind::PitchSpellSimple
                | SyntaxKind::PitchFrequency
                | SyntaxKind::PitchRatio
                | SyntaxKind::PitchEdo
                | SyntaxKind::PitchCents
        )
    }

    pub fn is_formal_pitch(&self) -> bool {
        matches!(self, SyntaxKind::PitchSustain | SyntaxKind::PitchRest)
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::*;

    #[test]
    fn test_lexer() {
        let path = Path::new("src/tests/sample.symi");
        let source = fs::read_to_string(&path).unwrap();
        let mut lex = SyntaxKind::lexer(&source);
        // output all tokens to `tests/sample_tokens.txt`
        let mut output = String::new();
        while let Some(token) = lex.next() {
            output.push_str(&format!(
                "{:?}: {:#?} @ {}..{}\n",
                token,
                lex.slice(),
                lex.span().start,
                lex.span().end
            ));
        }
        let out_path = path.with_file_name("sample_tokens.txt");
        fs::write(&out_path, output).unwrap();
    }
}
