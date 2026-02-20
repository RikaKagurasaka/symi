use std::sync::Arc;

use rowan::TextRange;

use crate::rowan::lexer::SyntaxKind;

/// 解析错误结构体，携带错误文本与范围。
///
/// 通常由解析器在遇到不符合预期的语法时生成，
/// 以便后续在 UI 中高亮或展示诊断信息。
///
/// # 示例
/// ```rust
/// use rowan::TextRange;
/// use symi::rowan::types::ParseError;
///
/// let err = ParseError::new("expected token", TextRange::new(0.into(), 1.into()));
/// assert_eq!(err.message, "expected token");
/// ```
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub range: TextRange,
}

impl ParseError {
    /// 创建一个新的解析错误。
    ///
    /// # 示例
    /// ```rust
    /// use rowan::TextRange;
    /// use symi::rowan::types::ParseError;
    ///
    /// let err = ParseError::new("unexpected", TextRange::new(0.into(), 1.into()));
    /// assert!(err.message.contains("unexpected"));
    /// ```
    pub fn new(message: impl Into<String>, range: TextRange) -> Self {
        Self {
            message: message.into(),
            range,
        }
    }
}

/// 词法 Token 的轻量结构体，包含种类、原始文本和范围。
///
/// 通常由词法分析器产生，并在语法分析阶段被消费。
///
/// # 示例
/// ```rust,ignore
/// use rowan::TextRange;
/// use symi::rowan::{lexer::SyntaxKind, types::Token};
///
/// let t = Token { kind: SyntaxKind::Comma, text: ",", range: TextRange::new(0.into(), 1.into()) };
/// assert_eq!(t.text, ",");
/// ```
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: SyntaxKind,
    pub source: Arc<str>,
    pub range: TextRange,
}

/// 解析事件枚举。
///
/// 解析器不会直接构建 `GreenNode`，而是先产出事件序列，
/// 由 `Sink` 统一消费并生成树结构。
///
/// # 示例
/// ```rust
/// use symi::rowan::{lexer::SyntaxKind, types::Event};
///
/// let ev = Event::StartNode { kind: SyntaxKind::NODE_ROOT, forward_parent: None };
/// if let Event::StartNode { kind, .. } = ev {
///     assert_eq!(kind, SyntaxKind::NODE_ROOT);
/// }
/// ```
#[derive(Debug, Clone)]
pub enum Event {
    StartNode {
        kind: SyntaxKind,
        forward_parent: Option<u32>,
    },
    FinishNode,
    Token {
        kind: Option<SyntaxKind>,
    },
    Tombstone,
}
