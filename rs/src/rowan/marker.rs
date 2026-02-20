use crate::rowan::{
    lexer::SyntaxKind,
    parser::{DropBomb, Parser},
    types::Event,
};

/// 解析阶段的“起始标记”。
///
/// 用于在事件流中占位一个将来会完成的语法节点。
/// 使用 `complete` 完成节点，或 `abandon` 放弃该节点。
///
/// # 示例
/// ```rust,ignore
/// use symi::rowan::{lexer::SyntaxKind, parser::Parser, marker::Marker};
///
/// // 仅展示用法示意：通过 Parser 创建 Marker，再完成它。
/// # let mut parser = Parser::new(vec![]);
/// let m = parser.start_node();
/// let _done = m.complete(&mut parser, SyntaxKind::NODE_ROOT);
/// ```
pub struct Marker {
    pos: usize,
    bomb: DropBomb,
}

impl Marker {
    /// 创建一个新的 `Marker`，供解析器内部使用。
    ///
    /// # 示例
    /// ```rust,ignore
    /// use symi::rowan::{marker::Marker, parser::Parser};
    /// # let mut parser = Parser::new(vec![]);
    /// let _m = parser.start_node();
    /// ```
    pub(crate) fn new(pos: usize) -> Self {
        Self {
            pos,
            bomb: DropBomb::new(),
        }
    }

    /// 完成该 `Marker`，并返回已完成的 `CompletedMarker`。
    ///
    /// # 示例
    /// ```rust,ignore
    /// use symi::rowan::{lexer::SyntaxKind, parser::Parser};
    /// # let mut parser = Parser::new(vec![]);
    /// let m = parser.start_node();
    /// let _done = m.complete(&mut parser, SyntaxKind::NODE_ROOT);
    /// ```
    pub fn complete(mut self, parser: &mut Parser, kind: SyntaxKind) -> CompletedMarker {
        self.bomb.defuse();
        debug_assert!(matches!(parser.events[self.pos], Event::Tombstone));

        parser.events[self.pos] = Event::StartNode {
            kind,
            forward_parent: None,
        };
        parser.events.push(Event::FinishNode);

        CompletedMarker { pos: self.pos }
    }

    /// 放弃该 `Marker`，不生成节点。
    ///
    /// # 示例
    /// ```rust,ignore
    /// use symi::rowan::parser::Parser;
    /// # let mut parser = Parser::new(vec![]);
    /// let m = parser.start_node();
    /// m.abandon(&mut parser);
    /// ```
    pub fn abandon(mut self, parser: &mut Parser) {
        self.bomb.defuse();
        parser.events[self.pos] = Event::Tombstone;
    }
}

/// 已完成节点的句柄。
///
/// 通过 `precede` 可在该节点前插入父节点（适用于需要事后包裹的语法结构）。
///
/// # 示例
/// ```rust,ignore
/// use symi::rowan::{lexer::SyntaxKind, parser::Parser};
/// # let mut parser = Parser::new(vec![]);
/// let m = parser.start_node();
/// let done = m.complete(&mut parser, SyntaxKind::NODE_NOTE);
/// let parent = done.precede(&mut parser);
/// let _ = parent.complete(&mut parser, SyntaxKind::NODE_NOTE_GROUP);
/// ```
pub struct CompletedMarker {
    pos: usize,
}

impl CompletedMarker {
    /// 在当前节点之前插入一个父节点，并返回新的 `Marker`。
    ///
    /// # 示例
    /// ```rust,ignore
    /// use symi::rowan::{lexer::SyntaxKind, parser::Parser};
    /// # let mut parser = Parser::new(vec![]);
    /// let done = parser.start_node().complete(&mut parser, SyntaxKind::NODE_NOTE);
    /// let parent = done.precede(&mut parser);
    /// let _ = parent.complete(&mut parser, SyntaxKind::NODE_NOTE_GROUP);
    /// ```
    pub fn precede(self, parser: &mut Parser) -> Marker {
        let new_pos = parser.events.len();
        parser.events.push(Event::Tombstone);

        let distance = new_pos - self.pos;
        assert!(distance <= u32::MAX as usize, "precede distance overflow");

        if let Event::StartNode { forward_parent, .. } = &mut parser.events[self.pos] {
            debug_assert!(forward_parent.is_none());
            *forward_parent = Some(distance as u32);
        } else {
            panic!("precede called on non-start node");
        }

        Marker::new(new_pos)
    }
}
