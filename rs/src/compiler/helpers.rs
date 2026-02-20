use rowan::NodeOrToken;

use crate::rowan::{
    lexer::SyntaxKind,
    parser::{SyntaxNode, SyntaxToken},
};

pub trait NodeOrTokenAsKind {
    fn kind(&self) -> SyntaxKind;
}

impl NodeOrTokenAsKind for NodeOrToken<SyntaxNode, SyntaxToken> {
    /// 获取该节点或标记的语法种类。
    ///
    /// # 示例
    /// ```rust,ignore
    /// use symi::rowan::{lexer::SyntaxKind, parser::Parser, types::NodeOrTokenAsKind};
    ///
    /// let mut parser = Parser::new(vec![]);
    /// let root = parser.parse();
    /// let kind = root.kind();
    /// assert_eq!(kind.into(), SyntaxKind::NODE_ROOT);
    /// ```
    fn kind(&self) -> SyntaxKind {
        match self {
            NodeOrToken::Node(n) => n.kind().into(),
            NodeOrToken::Token(t) => t.kind().into(),
        }
    }
}

pub trait SyntaxNodeEx {
    fn find_child_token_by_fn<P>(&self, predicate: P) -> Option<SyntaxToken>
    where
        P: Fn(&SyntaxToken) -> bool;
    fn find_child_node_by_fn<P>(&self, predicate: P) -> Option<SyntaxNode>
    where
        P: Fn(&SyntaxNode) -> bool;
    fn find_child_tokens_by_fn<P>(&self, predicate: P) -> Vec<SyntaxToken>
    where
        P: Fn(&SyntaxToken) -> bool;
    fn find_child_nodes_by_fn<P>(&self, predicate: P) -> Vec<SyntaxNode>
    where
        P: Fn(&SyntaxNode) -> bool;
}

impl SyntaxNodeEx for SyntaxNode {
    fn find_child_token_by_fn<P>(&self, predicate: P) -> Option<SyntaxToken>
    where
        P: Fn(&SyntaxToken) -> bool,
    {
        self.children_with_tokens()
            .filter_map(|n| n.into_token())
            .find(|t| predicate(t))
    }

    fn find_child_node_by_fn<P>(&self, predicate: P) -> Option<SyntaxNode>
    where
        P: Fn(&SyntaxNode) -> bool,
    {
        self.children()
            .find(|n| predicate(n))
    }

    fn find_child_tokens_by_fn<P>(&self, predicate: P) -> Vec<SyntaxToken>
    where
        P: Fn(&SyntaxToken) -> bool,
    {
        self.children_with_tokens()
            .filter_map(|n| n.into_token())
            .filter(|t| predicate(t))
            .collect()
    }

    fn find_child_nodes_by_fn<P>(&self, predicate: P) -> Vec<SyntaxNode>
    where
        P: Fn(&SyntaxNode) -> bool,
    {
        self.children()
            .filter(|n| predicate(n))
            .collect()
    }
}
