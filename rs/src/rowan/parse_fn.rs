use std::sync::Arc;

use crate::rowan::{
    lexer::SyntaxKind,
    parser::{Parse, Parser, parse},
};

/// 解析入口：构建语法树结构。
pub fn parse_source(source: Arc<str>) -> Parse {
    parse(source, parse_root)
}

/// 根节点解析函数（由 `parse` 调用）。
fn parse_root(parser: &mut Parser) {
    while let Some(tok) = parser.peek() {
        match tok {
            SyntaxKind::Whitespace | SyntaxKind::Comment => {
                unreachable!("trivia should be skipped in peek");
            }
            SyntaxKind::Identifier
                if parser.look_for_before(SyntaxKind::Equals, SyntaxKind::Newline) =>
            {
                parse_macro_def(parser);
            }
            SyntaxKind::Equals => {
                parse_normal_line(parser, true);
            }
            SyntaxKind::Newline => {
                parser.bump(); // consume newline
            }
            _ => {
                parse_normal_line(parser, false);
            }
        }
    }
}

macro_rules! SyntaxKindPitches {
    () => {
        SyntaxKind::PitchCents
            | SyntaxKind::PitchRatio
            | SyntaxKind::PitchFrequency
            | SyntaxKind::PitchEdo
            | SyntaxKind::PitchSpellOctave
            | SyntaxKind::PitchSpellSimple
            | SyntaxKind::PitchRest
            | SyntaxKind::PitchSustain
    };
}

/// 解析普通行（非宏定义行）。
fn parse_normal_line(parser: &mut Parser, is_ghost: bool) {
    let m = parser.start_node();
    if is_ghost {
        parser.eat(SyntaxKind::Equals); // consume '=' for ghost line
    }
    while let Some(tok) = parser.peek() {
        match tok {
            SyntaxKind::Newline => {
                parser.bump(); // consume newline
                break; // reach EOL
            }
            SyntaxKind::Comma | SyntaxKind::Quantize => {
                parser.bump(); // consume simple tokens
            }
            SyntaxKind::LAngle => {
                parse_base_pitch(parser);
            }
            SyntaxKind::LParen if parser.nth(1).is_some_and(|s| s.is_pitch_ratio()) => {
                parse_time_signature(parser);
            }
            SyntaxKind::LParen
                if parser.nth(1).is_some_and(|s| s.is_duration_fraction())
                    || parser.nth(1).is_some_and(|s| s.is_pitch_frequency()) =>
            {
                parse_bpm(parser);
            }
            SyntaxKindPitches!() | SyntaxKind::Identifier | SyntaxKind::Semicolon => {
                parse_note_group(parser);
            }
            _ => {
                parser.error("Unexpected token in normal line");
                parser.bump(); // consume to avoid infinite loop
            }
        }
    }
    m.complete(
        parser,
        if is_ghost {
            SyntaxKind::NODE_GHOST_LINE
        } else {
            SyntaxKind::NODE_NORMAL_LINE
        },
    );
}

fn parse_note_group(parser: &mut Parser) {
    let note_group_marker = parser.start_node();
    let mut is_group = false;
    let mut note_marker = None;
    while let Some(tok) = parser.peek() {
        match tok {
            SyntaxKindPitches!() => {
                note_marker.get_or_insert_with(|| parser.start_node());
                let chain_marker = parser.start_node();
                parser.bump(); // consume pitch token
                parse_pitch_chain_tail(parser);
                chain_marker.complete(parser, SyntaxKind::NODE_PITCH_CHAIN);
            }
            SyntaxKind::Identifier => {
                note_marker.get_or_insert_with(|| parser.start_node());
                let chain_marker = parser.start_node();
                let mm = parser.start_node();
                parser.bump(); // consume macro name
                parse_pitch_chain_tail(parser);
                mm.complete(parser, SyntaxKind::NODE_MACRO_INVOKE);
                chain_marker.complete(parser, SyntaxKind::NODE_PITCH_CHAIN);
            }
            SyntaxKind::Colon | SyntaxKind::Semicolon => {
                is_group = true;
                note_marker
                    .take()
                    .map(|m| m.complete(parser, SyntaxKind::NODE_NOTE));
                parser.bump(); // consume colon/semicolon
            }
            SyntaxKind::DurationCommas | SyntaxKind::DurationFraction => {
                parser.bump(); // consume duration token
            }

            SyntaxKind::Newline => {
                parser.error("unexpected end of line in note group");
                break; // end of line
            }
            _ => {
                break; // end of note group
            }
        }
    }
    // Complete any pending note if it is not empty
    note_marker
        .take()
        .map(|m| m.complete(parser, SyntaxKind::NODE_NOTE));
    if is_group {
        note_group_marker.complete(parser, SyntaxKind::NODE_NOTE_GROUP);
    } else {
        note_group_marker.abandon(parser);
    }
}

fn parse_pitch_chain_tail(parser: &mut Parser) {
    loop {
        while parser.eat(SyntaxKind::Plus) || parser.eat(SyntaxKind::PitchSustain) {}

        if !parser.eat(SyntaxKind::At) {
            break;
        }

        if parser
            .peek()
            .is_some_and(|k| k.is_pitch() || k.is_identifier())
        {
            parser.bump();
            continue;
        }
        parser.error("Expected pitch token or identifier after '@'");
        break;
    }
}

fn parse_bpm(parser: &mut Parser) {
    let m = parser.start_node();
    parser.expect(SyntaxKind::LParen); // consume '('
    if parser.eat(SyntaxKind::DurationFraction) {
        parser.expect(SyntaxKind::Equals);
    }
    parser.expect(SyntaxKind::PitchFrequency);
    parser.expect(SyntaxKind::RParen); // consume ')'
    m.complete(parser, SyntaxKind::NODE_BPM_DEF);
}

fn parse_time_signature(parser: &mut Parser) {
    let m = parser.start_node();
    parser.expect(SyntaxKind::LParen); // consume '('
    parser.expect(SyntaxKind::PitchRatio);
    parser.expect(SyntaxKind::RParen); // consume ')'
    m.complete(parser, SyntaxKind::NODE_TIME_SIGNATURE_DEF);
}

fn parse_base_pitch(parser: &mut Parser) {
    let m = parser.start_node();
    parser.expect(SyntaxKind::LAngle); // consume '<'
    let has_spell =
        parser.eat(SyntaxKind::PitchSpellOctave) || parser.eat(SyntaxKind::PitchSpellSimple);
    if has_spell {
        if parser.eat(SyntaxKind::Equals) {
            if parser.peek().is_some_and(|s| s.is_pitch() || s.is_identifier()) {
                let chain_marker = parser.start_node();
                parser.bump();
                parse_pitch_chain_tail(parser);
                chain_marker.complete(parser, SyntaxKind::NODE_PITCH_CHAIN);
            } else {
                parser.error("Expected pitch token after '=' in base pitch definition");
            }
        }
    } else if parser.peek().is_some_and(|s| s.is_pitch() || s.is_identifier()) {
        let chain_marker = parser.start_node();
        parser.bump();
        parse_pitch_chain_tail(parser);
        chain_marker.complete(parser, SyntaxKind::NODE_PITCH_CHAIN);
    } else {
        parser.error("Base pitch definition must contain a pitch token");
    }
    parser.expect(SyntaxKind::RAngle); // consume '>'
    m.complete(parser, SyntaxKind::NODE_BASE_PITCH_DEF);
}

fn parse_macro_def(parser: &mut Parser) {
    let m = parser.start_node();
    parser.expect(SyntaxKind::Identifier); // consume macro name
    parser.expect(SyntaxKind::Equals); // consume '='
    if parser.peek().is_some_and(|s| s.is_newline()) {
        parse_multi_line_macro_def(parser, m, false);
    } else if parser.peek().is_some_and(|s| s.is_pitch() || s.is_identifier()) {
        if parser.look_for_before(SyntaxKind::Colon, SyntaxKind::Newline) {
            parse_simple_macro_def(parser, m);
        } else {
            parse_alias_macro_def(parser, m);
        }
    } else {
        parse_multi_line_macro_def(parser, m, true);
    }
}

fn parse_alias_macro_def(parser: &mut Parser, m: super::marker::Marker) {
    let chain_marker = parser.start_node();
    if parser.peek().is_some_and(|s| s.is_pitch()) {
        parser.bump();
    } else if parser.peek().is_some_and(|s| s.is_identifier()) {
        let mm = parser.start_node();
        parser.bump();
        mm.complete(parser, SyntaxKind::NODE_MACRO_INVOKE);
    } else {
        parser.error("Alias macro definition must start with a pitch token or identifier");
        m.complete(parser, SyntaxKind::NODE_MACRODEF_ALIAS);
        return;
    }
    parse_pitch_chain_tail(parser);
    chain_marker.complete(parser, SyntaxKind::NODE_PITCH_CHAIN);

    while let Some(tok) = parser.peek() {
        if tok == SyntaxKind::Newline {
            break;
        }
        parser.error(format!("Unexpected token {:?} in alias macro definition", tok));
        parser.bump();
    }

    m.complete(parser, SyntaxKind::NODE_MACRODEF_ALIAS);
}

fn parse_simple_macro_def(parser: &mut Parser, m: super::marker::Marker) {
    let mut note_marker = None;
    while let Some(tok) = parser.peek() {
        match tok {
            SyntaxKindPitches!() => {
                note_marker.get_or_insert_with(|| parser.start_node());
                let chain_marker = parser.start_node();
                parser.bump(); // consume pitch token
                parse_pitch_chain_tail(parser);
                chain_marker.complete(parser, SyntaxKind::NODE_PITCH_CHAIN);
            }
            SyntaxKind::Identifier => {
                note_marker.get_or_insert_with(|| parser.start_node());
                let chain_marker = parser.start_node();
                let mm = parser.start_node();
                parser.bump(); // consume macro name
                parse_pitch_chain_tail(parser);
                mm.complete(parser, SyntaxKind::NODE_MACRO_INVOKE);
                chain_marker.complete(parser, SyntaxKind::NODE_PITCH_CHAIN);
            }
            SyntaxKind::Colon => {
                note_marker
                    .take()
                    .map(|marker| marker.complete(parser, SyntaxKind::NODE_NOTE));
                parser.bump(); // consume colon as note separator
            }
            SyntaxKind::Newline => {
                break; // reach EOL
            }
            _ => {
                parser.error(format!("Unexpected token {:?} in simple macro definition", tok));
                parser.bump(); // consume to avoid infinite loop
            }
        }
    }
    note_marker
        .take()
        .map(|marker| marker.complete(parser, SyntaxKind::NODE_NOTE));
    m.complete(parser, SyntaxKind::NODE_MACRODEF_SIMPLE);
}

fn parse_multi_line_macro_def(parser: &mut Parser, m: super::marker::Marker, is_single_line: bool) {
    if !is_single_line {
        parser.expect(SyntaxKind::Newline); // consume newline
    }
    let body_marker = parser.start_node();
    while let Some(tok) = parser.peek() {
        match tok {
            SyntaxKind::Newline => {
                parser.bump(); // consume newline
                break; // end of macro body
            }
            _ => {
                parse_normal_line(parser, false);
                if is_single_line {
                    break; // only one line in single-line macro body
                }
            }
        }
    }
    body_marker.complete(parser, SyntaxKind::NODE_MACRODEF_COMPLEX_BODY);
    m.complete(parser, SyntaxKind::NODE_MACRODEF_COMPLEX);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rowan::lexer::SyntaxKind;
    use std::{fs, path::Path};

    fn collect_kinds(root: &crate::rowan::parser::SyntaxNode) -> Vec<SyntaxKind> {
        root.descendants().map(|n| n.kind().into()).collect()
    }

    #[test]
    fn parse_empty_source_ok() {
        let result = parse_source(Arc::from(""));
        assert!(result.errors().is_empty());
        let root_kind: SyntaxKind = result.syntax_node().kind().into();
        assert_eq!(root_kind, SyntaxKind::NODE_ROOT);
    }

    #[test]
    fn parse_newline_only_ok() {
        let result = parse_source(Arc::from("\n"));
        assert!(result.errors().is_empty());
        let root_kind: SyntaxKind = result.syntax_node().kind().into();
        assert_eq!(root_kind, SyntaxKind::NODE_ROOT);
    }

    #[test]
    fn parse_assign_creates_ghost_line() {
        let result = parse_source(Arc::from("="));
        assert!(result.errors().is_empty());
        let root = result.syntax_node();
        let first_child = root.children().next().expect("expected a line node");
        let kind: SyntaxKind = first_child.kind().into();
        assert_eq!(kind, SyntaxKind::NODE_GHOST_LINE);
    }

    #[test]
    fn parse_simple_note_ok() {
        let result = parse_source(Arc::from("C4,"));
        assert!(result.errors().is_empty());
        let root = result.syntax_node();
        let line = root.children().next().expect("expected line node");
        let line_kind: SyntaxKind = line.kind().into();
        assert_eq!(line_kind, SyntaxKind::NODE_NORMAL_LINE);
        let note = line.children().find(|n| {
            let kind: SyntaxKind = n.kind().into();
            kind == SyntaxKind::NODE_NOTE
        });
        assert!(note.is_some());
    }

    #[test]
    fn parse_note_group_ok() {
        let result = parse_source(Arc::from("C4:D4,"));
        assert!(result.errors().is_empty());
        let root = result.syntax_node();
        let line = root.children().next().expect("expected line node");
        let group = line.children().find(|n| {
            let kind: SyntaxKind = n.kind().into();
            kind == SyntaxKind::NODE_NOTE_GROUP
        });
        assert!(group.is_some());
    }

    #[test]
    fn parse_macro_alias_def_ok() {
        let result = parse_source(Arc::from("foo = C4\n"));
        assert!(result.errors().is_empty());
        let root = result.syntax_node();
        let def = root.children().find(|n| {
            let kind: SyntaxKind = n.kind().into();
            kind == SyntaxKind::NODE_MACRODEF_ALIAS
        });
        assert!(def.is_some());
    }

    #[test]
    fn parse_macro_simple_def_pitch_chain_builds_note_nodes() {
        let result = parse_source(Arc::from("a = 3/2\nfoo = C4@a:D4\n"));
        assert!(result.errors().is_empty());
        let root = result.syntax_node();
        let has_simple_def = root.children().any(|n| {
            let kind: SyntaxKind = n.kind().into();
            kind == SyntaxKind::NODE_MACRODEF_SIMPLE
        });
        assert!(has_simple_def);
        let note_count = root
            .descendants()
            .filter(|n| {
                let kind: SyntaxKind = n.kind().into();
                kind == SyntaxKind::NODE_NOTE
            })
            .count();
        assert!(note_count >= 2);
    }

    #[test]
    fn parse_macro_complex_def_ok() {
        let result = parse_source(Arc::from("baz =\nC4,\n\n"));
        assert!(result.errors().is_empty());
        let root = result.syntax_node();
        let def = root.children().find(|n| {
            let kind: SyntaxKind = n.kind().into();
            kind == SyntaxKind::NODE_MACRODEF_COMPLEX
        });
        assert!(def.is_some());
    }

    #[test]
    fn parse_base_pitch_ok() {
        let result = parse_source(Arc::from("<C4=440>\n"));
        assert!(result.errors().is_empty());
        let root = result.syntax_node();
        let def = root.children().flat_map(|n| n.children()).find(|n| {
            let kind: SyntaxKind = n.kind().into();
            kind == SyntaxKind::NODE_BASE_PITCH_DEF
        });
        assert!(def.is_some());
    }

    #[test]
    fn parse_base_pitch_non_frequency_rhs_ok() {
        let result = parse_source(Arc::from("<C4=3/2>\n"));
        assert!(result.errors().is_empty());
        let root = result.syntax_node();
        let def = root.children().flat_map(|n| n.children()).find(|n| {
            let kind: SyntaxKind = n.kind().into();
            kind == SyntaxKind::NODE_BASE_PITCH_DEF
        });
        assert!(def.is_some());
    }

    #[test]
    fn parse_base_pitch_rhs_identifier_chain_ok() {
        let result = parse_source(Arc::from("a = 3/2@5/4\n<C4=a>\n"));
        assert!(result.errors().is_empty());
        let root = result.syntax_node();
        let has_chain = root.descendants().any(|n| {
            let kind: SyntaxKind = n.kind().into();
            kind == SyntaxKind::NODE_BASE_PITCH_DEF
                && n.children().any(|c| {
                    let ck: SyntaxKind = c.kind().into();
                    ck == SyntaxKind::NODE_PITCH_CHAIN
                })
        });
        assert!(has_chain);
    }

    #[test]
    fn parse_bpm_ok() {
        let result = parse_source(Arc::from("(120)\n"));
        assert!(result.errors().is_empty());
        let root = result.syntax_node();
        let def = root.children().flat_map(|n| n.children()).find(|n| {
            let kind: SyntaxKind = n.kind().into();
            kind == SyntaxKind::NODE_BPM_DEF
        });
        assert!(def.is_some());
    }

    #[test]
    fn parse_time_signature_ok() {
        let result = parse_source(Arc::from("(3/4)\n"));
        assert!(result.errors().is_empty());
        let root = result.syntax_node();
        let def = root.children().flat_map(|n| n.children()).find(|n| {
            let kind: SyntaxKind = n.kind().into();
            kind == SyntaxKind::NODE_TIME_SIGNATURE_DEF
        });
        assert!(def.is_some());
    }

    #[test]
    fn parse_pitch_chain_note_ok() {
        let result = parse_source(Arc::from("C4@3/2@100c,\n"));
        assert!(result.errors().is_empty());
        let root = result.syntax_node();
        let has_chain_node = root.descendants().any(|n| {
            let kind: SyntaxKind = n.kind().into();
            kind == SyntaxKind::NODE_PITCH_CHAIN
        });
        assert!(has_chain_node);
        let has_at = root.descendants_with_tokens().any(|nt| {
            nt.into_token()
                .is_some_and(|t| t.kind() == SyntaxKind::At.into())
        });
        assert!(has_at);
    }

    #[test]
    fn parse_pitch_chain_suffix_plus_minus_ok() {
        let result = parse_source(Arc::from("3/2++@4/3-,\n"));
        assert!(result.errors().is_empty());
        let root = result.syntax_node();
        let has_plus = root.descendants_with_tokens().any(|nt| {
            nt.into_token()
                .is_some_and(|t| t.kind() == SyntaxKind::Plus.into())
        });
        let has_minus = root.descendants_with_tokens().any(|nt| {
            nt.into_token()
                .is_some_and(|t| t.kind() == SyntaxKind::PitchSustain.into())
        });
        assert!(has_plus);
        assert!(has_minus);
    }

    #[test]
    fn parse_pitch_chain_macro_invoke_ok() {
        let result = parse_source(Arc::from("foo@C4@3/2,\n"));
        assert!(result.errors().is_empty());
        let root = result.syntax_node();
        let has_invoke = root.descendants().any(|n| {
            let kind: SyntaxKind = n.kind().into();
            kind == SyntaxKind::NODE_MACRO_INVOKE
        });
        assert!(has_invoke);
        let has_at = root.descendants_with_tokens().any(|nt| {
            nt.into_token()
                .is_some_and(|t| t.kind() == SyntaxKind::At.into())
        });
        assert!(has_at);
    }

    #[test]
    fn parse_pitch_chain_identifier_tail_ok() {
        let result = parse_source(Arc::from("m = 3/2\nC4@m,\n"));
        assert!(result.errors().is_empty());
        let root = result.syntax_node();
        let has_identifier = root.descendants_with_tokens().any(|nt| {
            nt.into_token()
                .is_some_and(|t| t.kind() == SyntaxKind::Identifier.into())
        });
        assert!(has_identifier);
    }

    #[test]
    fn parse_pitch_chain_trailing_at_reports_error() {
        let result = parse_source(Arc::from("C4@,\n"));
        assert!(!result.errors().is_empty());
    }

    #[test]
    fn parse_line_with_quantize_ok() {
        let result = parse_source(Arc::from("C4,{4}\n"));
        assert!(result.errors().is_empty());
        let root = result.syntax_node();
        let line = root.children().next().expect("expected line node");
        let line_kind: SyntaxKind = line.kind().into();
        assert_eq!(line_kind, SyntaxKind::NODE_NORMAL_LINE);
    }

    #[test]
    fn parse_duration_commas_ok() {
        let result = parse_source(Arc::from("C4[,,],\n"));
        assert!(result.errors().is_empty());
        let kinds = collect_kinds(&result.syntax_node());
        assert!(kinds.contains(&SyntaxKind::NODE_NOTE));
    }

    #[test]
    fn parse_mixed_program_ok() {
        let source = "foo = C4\nbar = C4:D4\n<C4=440>\n(120)\n(3/4)\nC4:D4,\n";
        let result = parse_source(Arc::from(source));
        assert!(result.errors().is_empty());
        let kinds = collect_kinds(&result.syntax_node());
        assert!(kinds.contains(&SyntaxKind::NODE_MACRODEF_ALIAS));
        assert!(kinds.contains(&SyntaxKind::NODE_MACRODEF_SIMPLE));
        assert!(kinds.contains(&SyntaxKind::NODE_BASE_PITCH_DEF));
        assert!(kinds.contains(&SyntaxKind::NODE_BPM_DEF));
        assert!(kinds.contains(&SyntaxKind::NODE_TIME_SIGNATURE_DEF));
        assert!(kinds.contains(&SyntaxKind::NODE_NOTE_GROUP));
    }

    #[test]
    fn parse_note_group_reports_error_on_eol() {
        let result = parse_source(Arc::from("C4\n"));
        assert!(!result.errors().is_empty());
    }

    #[test]
    fn dump_sample_tree() {
        let path = Path::new("src/tests/sample.symi");
        let source: Arc<str> =
            Arc::from(fs::read_to_string(path).expect("failed to read tests/sample.symi"));
        let result: Parse = parse_source(source.clone());

        let root = result.syntax_node();
        let mut output = String::new();
        format_node(&root, &source, 0, &mut output);
        let errors = result.errors();
        if !errors.is_empty() {
            output.push_str("\nParse Errors:\n");
            for err in errors {
                output.push_str(&format!("- {} at {:?}\n", err.message, err.range));
            }
        }

        let out_path = path.with_file_name("sample_tree.txt");
        fs::write(out_path, output).expect("failed to write tests/sample_tree.txt");
    }

    fn format_node(
        node: &crate::rowan::parser::SyntaxNode,
        source: &str,
        indent: usize,
        out: &mut String,
    ) {
        let kind: SyntaxKind = node.kind().into();
        let range = node.text_range();
        let text = slice_source(source, range);
        indent_line(indent, out);
        let start: usize = range.start().into();
        let end: usize = range.end().into();
        out.push_str(&format!(
            "NODE {:?} [{}, {}] {:?}\n",
            kind, start, end, text
        ));

        for element in node.children_with_tokens() {
            match element {
                rowan::NodeOrToken::Node(child) => {
                    format_node(&child, source, indent + 2, out);
                }
                rowan::NodeOrToken::Token(token) => {
                    let tkind: SyntaxKind = token.kind().into();
                    let trange = token.text_range();
                    let ttext = slice_source(source, trange);
                    indent_line(indent + 2, out);
                    let tstart: usize = trange.start().into();
                    let tend: usize = trange.end().into();
                    out.push_str(&format!(
                        "TOKEN {:?} [{}, {}] {:?}\n",
                        tkind, tstart, tend, ttext
                    ));
                }
            }
        }
    }

    fn slice_source(source: &str, range: rowan::TextRange) -> String {
        let start: usize = range.start().into();
        let end: usize = range.end().into();
        source.get(start..end).unwrap_or("").to_string()
    }

    fn indent_line(indent: usize, out: &mut String) {
        for _ in 0..indent {
            out.push(' ');
        }
    }
}
