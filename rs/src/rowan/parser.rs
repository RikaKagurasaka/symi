use std::{ops::Range, sync::Arc};

use logos::Logos;
use rowan::{
    GreenNode, Language, SyntaxElement as RowanSyntaxElement, SyntaxNode as RowanSyntaxNode,
    SyntaxToken as RowanSyntaxToken, TextRange, TextSize,
};

use crate::rowan::{
    lexer::SyntaxKind,
    marker::Marker,
    sink::Sink,
    types::{Event, ParseError, Token},
};

/// 语言标记类型：将 rowan 的语法 API 与本工程 `SyntaxKind` 绑定。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct SymiLanguage;

impl Language for SymiLanguage {
    type Kind = SyntaxKind;

    /// 将 rowan 的原始 kind 映射回 `SyntaxKind`。
    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        raw.into()
    }

    /// 将 `SyntaxKind` 映射为 rowan 的原始 kind。
    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        kind.into()
    }
}

pub type SyntaxNode = RowanSyntaxNode<SymiLanguage>;
pub type SyntaxToken = RowanSyntaxToken<SymiLanguage>;
pub type SyntaxElementRef = RowanSyntaxElement<SymiLanguage>;

/// 解析选项：控制解析驱动的基本行为。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseOptions {
    pub root_kind: SyntaxKind,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            root_kind: SyntaxKind::NODE_ROOT,
        }
    }
}

/// 解析入口：对 `source` 进行词法分析，并把可变 `Parser` 交给 `entry`。
pub fn parse<'src, F>(source: Arc<str>, entry: F) -> Parse
where
    F: FnOnce(&mut Parser),
{
    parse_with_options(source, ParseOptions::default(), entry)
}

/// 解析入口（带选项）：允许指定根节点类型。
pub fn parse_with_options<'src, F>(source: Arc<str>, options: ParseOptions, entry: F) -> Parse
where
    F: FnOnce(&mut Parser),
{
    let (tokens, lex_errors) = tokenize(source);
    let mut parser = Parser::new(tokens);

    let root_marker = parser.start_node();
    entry(&mut parser);
    parser.flush_remaining_tokens();
    root_marker.complete(&mut parser, options.root_kind);

    parser.finish(lex_errors)
}

/// 解析完成后的结果（绿色树 + 错误列表）。
#[derive(Debug, Clone)]
pub struct Parse {
    pub green_node: GreenNode,
    pub errors: Vec<ParseError>,
    pub tokens: Vec<Token>,
}

impl Parse {
    /// 获取根语法节点（`SyntaxNode`）。
    pub fn syntax_node(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.green_node.clone())
    }

    /// 获取内部的 `GreenNode` 引用。
    pub fn green_node(&self) -> &GreenNode {
        &self.green_node
    }

    /// 取出 `GreenNode` 所有权。
    pub fn into_green(self) -> GreenNode {
        self.green_node
    }

    /// 获取解析错误列表。
    pub fn errors(&self) -> &[ParseError] {
        &self.errors
    }
}

/// 事件驱动解析器：先记录事件，再由 `Sink` 构建绿色树。
pub struct Parser {
    pub(crate) tokens: Vec<Token>,
    pub(crate) significant_indices: Vec<usize>,
    pub(crate) cursor: usize,
    pub(crate) raw_cursor: usize,
    pub(crate) events: Vec<Event>,
    pub(crate) errors: Vec<ParseError>,
}

impl Parser {
    /// 创建新的解析器（内部使用）。
    fn new(tokens: Vec<Token>) -> Self {
        let significant_indices = tokens
            .iter()
            .enumerate()
            .filter_map(|(idx, token)| (!token.kind.is_trivia()).then_some(idx))
            .collect();

        Self {
            tokens,
            significant_indices,
            cursor: 0,
            raw_cursor: 0,
            events: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// 开始一个新的语法节点，返回 `Marker` 以便稍后完成。
    pub fn start_node(&mut self) -> Marker {
        let pos = self.events.len();
        self.events.push(Event::Tombstone);
        Marker::new(pos)
    }

    /// 消费当前语义 token（保持其原始 kind）。
    pub fn bump(&mut self) -> bool {
        self.bump_internal(None)
    }

    /// 消费当前语义 token，但将其记为新的 `SyntaxKind`。
    pub fn bump_as(&mut self, new_kind: SyntaxKind) -> bool {
        self.bump_internal(Some(new_kind))
    }

    /// 内部消费函数，可选重映射 `SyntaxKind`。
    fn bump_internal(&mut self, remap: Option<SyntaxKind>) -> bool {
        if self.is_eof() {
            return false;
        }

        let raw_index = self.significant_indices[self.cursor];
        self.cursor += 1;
        self.flush_trivia_until(raw_index);
        self.events.push(Event::Token { kind: remap });
        self.raw_cursor = raw_index + 1;
        true
    }

    /// 将 `raw_cursor` 前的琐碎 token 全部写入事件流。
    fn flush_trivia_until(&mut self, target_raw_index: usize) {
        while self.raw_cursor < target_raw_index {
            self.events.push(Event::Token { kind: None });
            self.raw_cursor += 1;
        }
    }

    /// 刷新剩余所有 token（含琐碎 token）。
    fn flush_remaining_tokens(&mut self) {
        self.flush_trivia_until(self.tokens.len());
    }

    /// 如果当前位置是指定种类，则消费并返回 `true`。
    pub fn eat(&mut self, kind: SyntaxKind) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    /// 返回当前位置的 `SyntaxKind`。如果当前是trivia，则跳过直到下一个语义 token。
    pub fn peek(&self) -> Option<SyntaxKind> {
        self.nth(0)
    }

    /// 期望当前位置为指定种类，否则记录解析错误。
    pub fn expect(&mut self, kind: SyntaxKind) {
        if !self.eat(kind) {
            self.error(format!("expected {:?}", kind));
        }
    }
    /// 向前查看第 `n` 个语义 token 的种类。
    pub fn nth(&self, n: usize) -> Option<SyntaxKind> {
        self.significant_indices
            .get(self.cursor + n)
            .map(|&idx| self.tokens[idx].kind)
    }

    pub fn look_for_before(&self, look_for: SyntaxKind, before: SyntaxKind) -> bool {
        let mut offset = 0;
        while let Some(kind) = self.nth(offset) {
            if kind == before {
                return false;
            }
            if kind == look_for {
                return true;
            }
            offset += 1;
        }
        false
    }

    /// 判断当前位置是否为指定种类。
    pub fn at(&self, kind: SyntaxKind) -> bool {
        self.nth(0) == Some(kind)
    }

    /// 判断当前位置是否为任意一种 `kinds`。
    pub fn at_any(&self, kinds: &[SyntaxKind]) -> bool {
        kinds.iter().copied().any(|kind| self.at(kind))
    }

    /// 是否到达语义 token 末尾。
    pub fn is_eof(&self) -> bool {
        self.cursor >= self.significant_indices.len()
    }

    /// 记录一个解析错误，范围由 `current_range` 提供。
    pub fn error(&mut self, message: impl Into<String>) {
        let range = self.current_range();
        self.errors.push(ParseError::new(message, range));
    }

    /// 计算当前“指针”位置的文本范围。
    fn current_range(&self) -> TextRange {
        if let Some(&idx) = self.significant_indices.get(self.cursor) {
            return self.tokens[idx].range;
        }

        if let Some(token) = self.tokens.get(self.raw_cursor) {
            return token.range;
        }

        self.tokens
            .last()
            .map(|token| TextRange::new(token.range.end(), token.range.end()))
            .unwrap_or_else(|| TextRange::new(TextSize::from(0), TextSize::from(0)))
    }

    /// 结束解析：刷新剩余 token，构建绿色树，并汇总错误。
    fn finish(mut self, mut external_errors: Vec<ParseError>) -> Parse {
        self.flush_remaining_tokens();
        external_errors.extend(self.errors.into_iter());
        let sink = Sink::new(self.tokens.clone(), self.events);
        let green = sink.finish();

        Parse {
            tokens: self.tokens,
            green_node: green,
            errors: external_errors,
        }
    }
}

/// 标记“必须完成/放弃”的资源守卫。
///
/// 用于在 `Marker` 被丢弃前强制其完成或放弃，否则在 debug 构建中触发断言。
#[derive(Debug)]
pub struct DropBomb {
    armed: bool,
}

impl DropBomb {
    /// 创建一个新的 `DropBomb`。
    ///
    /// # 示例
    /// ```rust,ignore
    /// use symi::rowan::parser::DropBomb;
    /// let _bomb = DropBomb::new();
    /// ```
    pub(crate) fn new() -> Self {
        Self { armed: true }
    }

    /// 解除武装，允许安全地被丢弃。
    pub(crate) fn defuse(&mut self) {
        self.armed = false;
    }
}

impl Drop for DropBomb {
    fn drop(&mut self) {
        debug_assert!(
            !self.armed,
            "marker must be completed or abandoned before being dropped"
        );
    }
}

/// 对源文本进行词法分析，返回 Token 列表和词法错误列表。
fn tokenize(source: Arc<str>) -> (Vec<Token>, Vec<ParseError>) {
    let mut lexer = SyntaxKind::lexer(source.as_ref());
    let mut tokens = Vec::new();
    let mut errors = Vec::new();

    while let Some(tok) = lexer.next() {
        if let Ok(kind) = tok {
            let span = lexer.span();
            tokens.push(Token {
                kind,
                source: source.clone(),
                range: to_text_range(span),
            });
        } else {
            // 词法错误
            let span = lexer.span();
            let text_slice = &source[span.clone()];
            let text_range = to_text_range(span.clone());
            let message = format!("Unrecognized token: {:?}", text_slice);
            let error = ParseError::new(message, text_range);
            tokens.push(Token {
                kind: SyntaxKind::Error,
                source: source.clone(),
                range: text_range,
            });
            errors.push(error);
        }
    }

    (tokens, errors)
}

/// 将字节范围转换为 `TextRange`。
fn to_text_range(span: Range<usize>) -> TextRange {
    TextRange::new(to_text_size(span.start), to_text_size(span.end))
}

/// 将 `usize` 转换为 `TextSize`。
fn to_text_size(value: usize) -> TextSize {
    TextSize::from(value as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_empty_root() {
        let parse = parse(Arc::from("   "), |_| {});
        assert!(parse.errors().is_empty());
        let root_kind: SyntaxKind = parse.syntax_node().kind().into();
        assert_eq!(root_kind, SyntaxKind::NODE_ROOT);
    }
}
