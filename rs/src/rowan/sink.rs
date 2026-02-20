use rowan::{GreenNode, GreenNodeBuilder};

use crate::rowan::{
    lexer::SyntaxKind,
    types::{Event, Token},
};

/// 事件下沉器：将解析事件转换为 `GreenNode`。
///
/// 解析器阶段只记录 `Event` 序列，这里负责“回放”事件，
/// 使用 `GreenNodeBuilder` 创建最终的 rowan 绿色树。
///
/// # 示例
/// ```rust,ignore
/// use symi::rowan::{lexer::SyntaxKind, sink::Sink, types::{Event, Token}};
/// use rowan::TextRange;
///
/// let tokens = vec![Token { kind: SyntaxKind::Comma, text: ",", range: TextRange::new(0.into(), 1.into()) }];
/// let events = vec![
///     Event::StartNode { kind: SyntaxKind::NODE_ROOT, forward_parent: None },
///     Event::Token { kind: None },
///     Event::FinishNode,
/// ];
/// let green = Sink::new(tokens, events).finish();
/// let root = rowan::SyntaxNode::<crate::rowan::parser::SymiLanguage>::new_root(green);
/// assert_eq!(root.kind().into(), SyntaxKind::NODE_ROOT);
/// ```
pub(crate) struct Sink {
    tokens: Vec<Token>,
    events: Vec<Event>,
    builder: GreenNodeBuilder<'static>,
    token_cursor: usize,
}

impl Sink {
    /// 创建新的 `Sink`。
    ///
    /// # 示例
    /// ```rust,ignore
    /// use symi::rowan::{sink::Sink, types::Event};
    /// let sink = Sink::new(Vec::new(), Vec::new());
    /// let _ = sink;
    /// ```
    pub(crate) fn new(tokens: Vec<Token>, events: Vec<Event>) -> Self {
        Self {
            tokens,
            events,
            builder: GreenNodeBuilder::new(),
            token_cursor: 0,
        }
    }

    /// 回放事件并构建最终的 `GreenNode`。
    ///
    /// # 示例
    /// ```rust,ignore
    /// use symi::rowan::{lexer::SyntaxKind, sink::Sink, types::{Event, Token}};
    /// use rowan::TextRange;
    ///
    /// let tokens = vec![Token { kind: SyntaxKind::Comma, text: ",", range: TextRange::new(0.into(), 1.into()) }];
    /// let events = vec![
    ///     Event::StartNode { kind: SyntaxKind::NODE_ROOT, forward_parent: None },
    ///     Event::Token { kind: None },
    ///     Event::FinishNode,
    /// ];
    /// let green = Sink::new(tokens, events).finish();
    /// assert!(!green.children().is_empty());
    /// ```
    pub(crate) fn finish(mut self) -> GreenNode {
        for idx in 0..self.events.len() {
            let event = std::mem::replace(&mut self.events[idx], Event::Tombstone);
            match event {
                Event::StartNode {
                    kind,
                    forward_parent,
                } => {
                    self.start_with_forward_parents(idx, kind, forward_parent);
                }
                Event::FinishNode => self.builder.finish_node(),
                Event::Token { kind } => {
                    let token = &self.tokens[self.token_cursor];
                    self.token_cursor += 1;
                    let final_kind = kind.unwrap_or(token.kind);
                    self.builder
                        .token(final_kind.into(), &token.source[token.range]);
                }
                Event::Tombstone => {}
            }
        }

        self.builder.finish()
    }

    /// 处理 `forward_parent` 链，确保 `precede` 产生的父节点按正确顺序打开。
    ///
    /// # 示例
    /// ```rust,ignore
    /// use symi::rowan::{lexer::SyntaxKind, sink::Sink, types::{Event, Token}};
    /// use rowan::TextRange;
    ///
    /// let tokens = vec![Token { kind: SyntaxKind::Comma, text: ",", range: TextRange::new(0.into(), 1.into()) }];
    /// let events = vec![
    ///     Event::StartNode { kind: SyntaxKind::NODE_NOTE, forward_parent: Some(1) },
    ///     Event::StartNode { kind: SyntaxKind::NODE_NOTE_GROUP, forward_parent: None },
    ///     Event::Token { kind: None },
    ///     Event::FinishNode,
    ///     Event::FinishNode,
    /// ];
    /// let green = Sink::new(tokens, events).finish();
    /// let root = rowan::SyntaxNode::<crate::rowan::parser::SymiLanguage>::new_root(green);
    /// assert_eq!(root.first_child().unwrap().kind().into(), SyntaxKind::NODE_NOTE_GROUP);
    /// ```
    pub(crate) fn start_with_forward_parents(
        &mut self,
        mut idx: usize,
        kind: SyntaxKind,
        mut forward_parent: Option<u32>,
    ) {
        // Chain start nodes so that `precede` can retroactively insert parents.
        let mut kinds = Vec::with_capacity(4);
        kinds.push(kind);

        while let Some(steps) = forward_parent {
            idx += steps as usize;
            let event = std::mem::replace(&mut self.events[idx], Event::Tombstone);
            if let Event::StartNode {
                kind,
                forward_parent: next,
            } = event
            {
                kinds.push(kind);
                forward_parent = next;
            } else {
                break;
            }
        }

        for kind in kinds.into_iter().rev() {
            self.builder.start_node(kind.into());
        }
    }
}
