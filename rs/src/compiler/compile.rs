use std::{
    collections::HashMap,
    mem::take,
    ops::Neg,
    vec,
};

use rowan::{NodeOrToken, TextRange};

use crate::{
    compiler::{
        helpers::SyntaxNodeEx,
        rational::Rational32,
        types::{
            CompileEvent, CompileState, Diagnostic, DiagnosticLevel, EventBody, MacroRegistry,
            Note, Pitch, TimeStamp, freq2spell,
        },
    },
    rowan::{
        lexer::SyntaxKind,
        parser::{SyntaxNode, SyntaxToken},
    },
};

pub struct Compiler {
    pub diagnostics: Vec<Diagnostic>,
    pub macros: MacroRegistry,
    pub state: CompileState,
    pub events: Vec<CompileEvent>,
}

impl Compiler {
    pub fn new() -> Self {
        let macros = MacroRegistry::default();
        let state = CompileState::new();
        Self {
            diagnostics: Vec::new(),
            macros,
            state,
            events: vec![],
        }
    }

    fn reset_ticks(&mut self) {
        if self.state.time.ticks.numer() > &0 {
            self.state.time.bars += 1;
            self.state.time.ticks = Rational32::new(0, *self.state.quantize.denom());
            self.push_event(
                EventBody::NewMeasure(self.state.time.bars),
                TextRange::default(),
            );
        }
    }

    pub fn compile(&mut self, tree: &SyntaxNode) {
        for child in tree.children_with_tokens() {
            match child {
                NodeOrToken::Node(node) => match node.kind() {
                    SyntaxKind::NODE_MACRODEF_ALIAS
                    |
                    SyntaxKind::NODE_MACRODEF_SIMPLE
                    | SyntaxKind::NODE_MACRODEF_COMPLEX => {
                        self.compile_macro_def(&node);
                    }
                    SyntaxKind::NODE_NORMAL_LINE | SyntaxKind::NODE_GHOST_LINE => {
                        self.compile_normal_line(&node);
                    }
                    SyntaxKind::Newline => {
                        // Ignore top-level newlines
                    }
                    _ => {
                        self.error(
                            format!("Unexpected node kind: {:?}", node.kind()),
                            node.text_range(),
                        );
                    }
                },
                NodeOrToken::Token(token) => {
                    if !token.kind().is_trivia() && !token.kind().is_newline() {
                        self.error(
                            format!("Unexpected token: {}", token.text()),
                            token.text_range(),
                        );
                    }
                }
            }
            self.reset_ticks();
        }
        self.finalize_negative_duration_notes();
        self.finalize_sustain_notes();
    }

    fn compile_normal_line(&mut self, node: &SyntaxNode) {
        debug_assert!(node.kind().is_node_normal_line() || node.kind().is_node_ghost_line());
        let is_ghost = node.kind().is_node_ghost_line();
        let start_time_stamp = if is_ghost {
            Some(self.state.time.clone())
        } else {
            None
        };
        for child in node.children_with_tokens() {
            match child {
                NodeOrToken::Node(n) => match n.kind().into() {
                    SyntaxKind::NODE_BPM_DEF => self.compile_bpm_def(&n),
                    SyntaxKind::NODE_TIME_SIGNATURE_DEF => self.compile_time_signature_def(&n),
                    SyntaxKind::NODE_BASE_PITCH_DEF => self.compile_base_pitch_def(&n),
                    SyntaxKind::NODE_NOTE_GROUP | SyntaxKind::NODE_NOTE => {
                        self.compile_note_group(&n)
                    }
                    _ => {
                        self.error(
                            format!("Unexpected node in line: {:?}", n.kind()),
                            n.text_range(),
                        );
                    }
                },
                NodeOrToken::Token(t) => match t.kind().into() {
                    SyntaxKind::Quantize => {
                        if let Some(dur) = self.parse_duration_fraction(&t) {
                            self.state.quantize = dur;
                            self.push_event(EventBody::QuantizeDef(dur), t.text_range());
                        }
                    }
                    SyntaxKind::Comma => {
                        // advance time by quantize
                        self.state.time = self
                            .state
                            .time
                            .add_duration(self.state.quantize, &self.state)
                            .reduct_to_quantize(self.state.quantize);
                    }
                    SyntaxKind::Newline
                    | SyntaxKind::Whitespace
                    | SyntaxKind::Comment
                    | SyntaxKind::Equals => {
                        // Ignore newlines within lines
                    }
                    _ => {
                        self.error(
                            format!("Unexpected token in line: {}", t.text()),
                            t.text_range(),
                        );
                    }
                },
            }
        }
        // check if current tick equals time signature or zero
        if self.state.time.ticks > Rational32::zero()
            && self.state.time.ticks != self.state.time_signature
        {
            self.warn(
                "Line ended but current ticks do not align with time signature".to_string(),
                node.text_range(),
            );
        }

        if let Some(ts) = start_time_stamp {
            self.state.time = ts;
        }
    }

    fn compile_macro_def(&mut self, node: &SyntaxNode) {
        debug_assert!(
            node.kind().is_node_macrodef_alias()
                ||
            node.kind().is_node_macrodef_simple()
                || node.kind().is_node_macrodef_complex()
        );
        let ident_tok = node
            .find_child_token_by_fn(|t| t.kind().is_identifier())
            .expect("Macro definition must have an identifier token");
        let note_kind = node.kind();
        match note_kind {
            SyntaxKind::NODE_MACRODEF_ALIAS => {
                let Some(chain_node) =
                    node.find_child_node_by_fn(|n| n.kind().is_node_pitch_chain())
                else {
                    self.error(
                        "Alias macro definition must contain a pitch chain".to_string(),
                        node.text_range(),
                    );
                    return;
                };
                let chain_tokens: Vec<SyntaxToken> = chain_node
                    .descendants_with_tokens()
                    .filter_map(|nt| nt.into_token())
                    .filter(|t| {
                        t.kind().is_pitch()
                            || t.kind().is_formal_pitch()
                            || t.kind().is_identifier()
                            || t.kind().is_at()
                            || t.kind().is_plus()
                    })
                    .collect();
                if let Some(note) = self.parse_base_pitch_rhs_chain_tokens(
                    &chain_tokens,
                    chain_node.text_range(),
                ) {
                    self.macros
                        .alias_macros
                        .insert(ident_tok.text().to_string(), note.pitch_chain);
                }
            }
            SyntaxKind::NODE_MACRODEF_SIMPLE => {
                let mut pitches: Vec<Note> = Vec::new();
                for child in node.children() {
                    if !child.kind().is_node_note() {
                        continue;
                    }
                    let mut has_chain = false;
                    for chain in child.children().filter(|n| n.kind().is_node_pitch_chain()) {
                        has_chain = true;
                        let chain_tokens: Vec<SyntaxToken> = chain
                            .descendants_with_tokens()
                            .filter_map(|nt| nt.into_token())
                            .filter(|t| {
                                t.kind().is_pitch()
                                    || t.kind().is_formal_pitch()
                                    || t.kind().is_identifier()
                                    || t.kind().is_at()
                                    || t.kind().is_plus()
                            })
                            .collect();
                        if chain_tokens.is_empty() {
                            self.error(
                                "Simple macro note must contain a pitch chain".to_string(),
                                chain.text_range(),
                            );
                            continue;
                        }
                        if let Some(note) = self.parse_pitch_chain_tokens(
                            &chain_tokens,
                            false,
                            chain.text_range(),
                        ) {
                            pitches.push(note);
                        }
                    }
                    if !has_chain {
                        self.error(
                            "Simple macro note must contain a pitch chain".to_string(),
                            child.text_range(),
                        );
                    }
                }
                self.macros
                    .simple_macros
                    .insert(ident_tok.text().to_string(), pitches);
            }
            SyntaxKind::NODE_MACRODEF_COMPLEX => {
                let saved_state = CompileState {
                    time: self.state.time,
                    base_note: self.state.base_note,
                    base_frequency: self.state.base_frequency,
                    time_signature: self.state.time_signature,
                    beat_duration: self.state.beat_duration,
                    bpm: self.state.bpm,
                    quantize: self.state.quantize,
                    edo_def: self.state.edo_def,
                };
                let saved_events = take(&mut self.events);

                self.state = CompileState {
                    time: TimeStamp {
                        seconds: 0.0,
                        bars: 0,
                        ticks: Rational32::new(0, *self.state.quantize.denom()),
                    },
                    base_note: saved_state.base_note,
                    base_frequency: saved_state.base_frequency,
                    time_signature: saved_state.time_signature,
                    beat_duration: saved_state.beat_duration,
                    bpm: saved_state.bpm,
                    quantize: saved_state.quantize,
                    edo_def: saved_state.edo_def,
                };

                let node_body = node
                    .find_child_node_by_fn(|n| n.kind().is_node_macrodef_complex_body())
                    .expect("Macro complex definition must have a body node");
                for node in node_body.children() {
                    self.compile_normal_line(&node);
                    self.reset_ticks();
                }

                let compiled_events = take(&mut self.events);
                self.macros.complex_macros.insert(
                    ident_tok.text().to_string(),
                    compiled_events,
                );
                self.state = saved_state;
                self.events = saved_events;
            }
            _ => {
                self.error(
                    format!("Unexpected macro definition kind: {:?}", note_kind),
                    node.text_range(),
                );
            }
        }
    }

    fn compile_time_signature_def(&mut self, n: &SyntaxNode) {
        debug_assert!(n.kind().is_node_time_signature_def());
        let duration_token = n
            .find_child_token_by_fn(|t| t.kind().is_pitch_ratio())
            .expect("Time signature definition must have a pitch ratio token (as ./. format)");
        let parts = duration_token.text().split('/').collect::<Vec<&str>>();
        if parts.len() == 2 {
            let numerator = parts[0].parse::<i32>().ok();
            let denominator = parts[1].parse::<i32>().ok();

            if let (Some(n), Some(d)) = (numerator, denominator) {
                if d == 0 {
                    self.error(
                        format!("Denominator of time signature cannot be zero: {}", d),
                        duration_token.text_range(),
                    );
                    return;
                }
                // if denominator is not pow of 2, issue warning
                if d.reverse_bits() & (d - 1) != 0 {
                    self.warn(
                        format!(
                            "Denominator of time signature is not a power of 2 but {}, which is discouraged",
                            d
                        ),
                        duration_token.text_range(),
                    );
                }

                let time_signature = Rational32::new(n, d);
                self.state.time_signature = time_signature;
                self.push_event(
                    EventBody::TimeSignatureDef(time_signature),
                    duration_token.text_range(),
                );
            } else {
                self.error(
                    format!("Invalid time signature format: {}", duration_token.text()),
                    duration_token.text_range(),
                );
            }
        } else {
            self.error(
                format!("Invalid time signature format: {}", duration_token.text()),
                duration_token.text_range(),
            );
        }
    }

    fn compile_bpm_def(&mut self, n: &SyntaxNode) {
        debug_assert!(n.kind().is_node_bpm_def());
        let duration_token = n.find_child_token_by_fn(|t| t.kind().is_duration_fraction());
        let Some(bpm_token) = n.find_child_token_by_fn(|t| t.kind().is_pitch_frequency()) else {
            self.error(
                "BPM definition must have a number token".to_string(),
                n.text_range(),
            );
            return;
        };
        if let Some(bpm) = bpm_token.text().parse::<f32>().ok() {
            if let Some(dur_tok) = duration_token {
                if let Some(dur) = self.parse_duration_fraction(&dur_tok) {
                    let beat_duration = dur;
                    self.state.beat_duration = beat_duration;
                    self.push_event(
                        EventBody::BeatDurationDef(beat_duration),
                        dur_tok.text_range(),
                    );
                }
            }
            self.state.bpm = bpm;
            self.push_event(EventBody::BPMDef(bpm), bpm_token.text_range());
        } else {
            self.error(
                format!("Invalid BPM value: {}", bpm_token.text()),
                bpm_token.text_range(),
            );
        }
    }

    fn parse_duration_fraction(&mut self, t: &SyntaxToken) -> Option<Rational32> {
        let rs = (|| {
            debug_assert!(t.kind().is_duration_fraction() || t.kind().is_quantize());
            let text = t.text().trim_matches(&['[', ']', '{', '}']); //also trim '{' '}'
            let parts: Vec<&str> = text.split(':').collect();
            let numerator: i32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
            let denominator: i32 = parts[0].parse().ok()?;
            if denominator != 0 {
                return Some(Rational32::new(numerator, denominator));
            }
            None
        })();
        if rs.is_none() {
            self.error(
                format!("Invalid duration format: {}", t.text()),
                t.text_range(),
            );
        }
        rs
    }

    fn parse_duration_commas(&mut self, t: &SyntaxToken) -> Option<u32> {
        debug_assert!(t.kind().is_duration_commas());
        let text = t.text();
        let count = text.chars().filter(|&c| c == ',').count() as u32;
        Some(count)
    }

    fn compile_base_pitch_def(&mut self, n: &SyntaxNode) {
        debug_assert!(n.kind().is_node_base_pitch_def());
        let pitch_spell = n
            .find_child_token_by_fn(|t| {
                t.kind().is_pitch_spell_octave() || t.kind().is_pitch_spell_simple()
            })
            .and_then(|t| self.parse_pitch(&t, false));

        let pitch_ref = n
            .find_child_node_by_fn(|child| child.kind().is_node_pitch_chain())
            .and_then(|chain_node| {
                let chain_tokens: Vec<SyntaxToken> = chain_node
                    .descendants_with_tokens()
                    .filter_map(|nt| nt.into_token())
                    .filter(|t| {
                        t.kind().is_pitch()
                            || t.kind().is_formal_pitch()
                            || t.kind().is_identifier()
                            || t.kind().is_at()
                            || t.kind().is_plus()
                    })
                    .collect();
                self.parse_base_pitch_rhs_chain_tokens(&chain_tokens, chain_node.text_range())
            });

        if let Some(spell) = pitch_spell {
            self.state.base_note = match spell.pitch_chain.first().copied() {
                Some(Pitch::SpellOctave(s)) => s,
                Some(Pitch::SpellSimple(s)) => s,
                _ => unreachable!("Base pitch must be a spell"),
            };
            self.state.base_frequency = pitch_ref.map(|p| p.freq).unwrap_or(spell.freq);
            self.push_event(EventBody::BaseNoteDef(self.state.base_note), n.text_range());
            self.push_event(
                EventBody::BaseFequencyDef(self.state.base_frequency),
                n.text_range(),
            );
        } else {
            if let Some(pitch_ref) = pitch_ref {
                self.state.base_note = freq2spell(pitch_ref.freq, &self.state);
                self.state.base_frequency = pitch_ref.freq;
                self.push_event(EventBody::BaseNoteDef(self.state.base_note), n.text_range());
                self.push_event(
                    EventBody::BaseFequencyDef(self.state.base_frequency),
                    n.text_range(),
                );
            } else {
                self.error(
                    "Base pitch definition must have either a pitch spell or pitch chain reference"
                        .to_string(),
                    n.text_range(),
                );
            }
        }
    }

    fn parse_pitch_chain_ident_as_chain_for_base_rhs(
        &mut self,
        t: &SyntaxToken,
    ) -> Option<Vec<Pitch>> {
        debug_assert!(t.kind().is_identifier());
        let ident = t.text().to_string();

        if let Some(chain) = self.macros.alias_macros.get(ident.as_str()) {
            return Some(chain.clone());
        }

        if self.macros.simple_macros.contains_key(ident.as_str()) {
            self.error(
                format!(
                    "Identifier in base pitch RHS must resolve to an alias macro: {}",
                    ident
                ),
                t.text_range(),
            );
            return None;
        }

        if self.macros.complex_macros.contains_key(ident.as_str()) {
            self.error(
                format!(
                    "Identifier in base pitch RHS cannot resolve to a complex macro: {}",
                    ident
                ),
                t.text_range(),
            );
            return None;
        }

        self.error(
            format!("Undefined identifier in base pitch RHS: {}", ident),
            t.text_range(),
        );
        None
    }

    fn parse_base_pitch_rhs_chain_tokens(
        &mut self,
        tokens: &[SyntaxToken],
        range: TextRange,
    ) -> Option<Note> {
        if tokens.is_empty() {
            return None;
        }

        let mut pitch_atoms: Vec<Pitch> = Vec::new();
        let mut expect_pitch = true;

        for token in tokens {
            if expect_pitch {
                if token.kind().is_pitch() || token.kind().is_formal_pitch() {
                    let pitch = self.parse_pitch_atom(token, false)?;
                    pitch_atoms.push(pitch);
                    expect_pitch = false;
                } else if token.kind().is_identifier() {
                    let chain = self.parse_pitch_chain_ident_as_chain_for_base_rhs(token)?;
                    if chain.is_empty() {
                        self.error(
                            "Identifier in base pitch RHS cannot resolve to an empty pitch chain"
                                .to_string(),
                            token.text_range(),
                        );
                        return None;
                    }
                    pitch_atoms.extend(chain);
                    expect_pitch = false;
                } else {
                    self.error(
                        format!("Expected pitch token, got: {}", token.text()),
                        token.text_range(),
                    );
                    return None;
                }
            } else if token.kind().is_at() {
                expect_pitch = true;
            } else if token.kind().is_plus() {
                pitch_atoms.push(Pitch::Ratio(Rational32::new(2, 1)));
            } else if token.kind().is_pitch_sustain() {
                pitch_atoms.push(Pitch::Ratio(Rational32::new(1, 2)));
            } else {
                self.error(
                    format!("Expected '@' in pitch chain, got: {}", token.text()),
                    token.text_range(),
                );
                return None;
            }
        }

        if expect_pitch {
            self.error("Pitch chain cannot end with '@'".to_string(), range);
            return None;
        }

        self.eval_pitch_chain_pitches(&pitch_atoms, range)
    }

    fn parse_pitch_atom(&mut self, t: &SyntaxToken, allow_formal: bool) -> Option<Pitch> {
        debug_assert!(t.kind().is_pitch() || t.kind().is_formal_pitch());
        if !allow_formal {
            if t.kind().is_formal_pitch() {
                self.error(
                    format!("Formal pitch not allowed here: {}", t.text()),
                    t.text_range(),
                );
                return None;
            }
        }
        let text = t.text();
        match t.kind() {
            SyntaxKind::PitchSpellOctave => Pitch::parse_spell_octave(text),
            SyntaxKind::PitchSpellSimple => Pitch::parse_spell_simple(text),
            SyntaxKind::PitchFrequency => {
                // handle edo grammar sugar: if edo_def is set and the token text is an integer, parse it as edo and convert to frequency
                if self.state.edo_def == 0 || text.contains('.') {
                    if text
                        .parse::<f32>()
                        .ok()
                        .filter(|&f| f >= 1.0 && f < 1e8)
                        .is_some()
                    {
                        self.state.edo_def = 0;
                        Pitch::parse_fequency(text)
                    } else {
                        self.error(format!("Invalid frequency value: {}", text), t.text_range());
                        None
                    }
                } else {
                    Pitch::parse_edo(format!("{}\\{}", text, self.state.edo_def).as_str())
                }
            }
            SyntaxKind::PitchRatio => Pitch::parse_ratio(text),
            SyntaxKind::PitchEdo => {
                let p = Pitch::parse_edo(text);
                if let Some(Pitch::Edo(r)) = p {
                    self.state.edo_def = *r.denom() as u16;
                }
                p
            }
            SyntaxKind::PitchCents => Pitch::parse_cents(text),
            SyntaxKind::PitchRest => Some(Pitch::Rest),
            SyntaxKind::PitchSustain => Some(Pitch::Sustain),
            _ => {
                self.error(format!("Invalid pitch token: {}", text), t.text_range());
                None
            }
        }
    }

    fn parse_pitch(&mut self, t: &SyntaxToken, allow_formal: bool) -> Option<Note> {
        self.parse_pitch_atom(t, allow_formal)
            .map(|pitch| Note::from_pitch(pitch, &self.state))
    }

    fn parse_pitch_chain_ident_as_chain(&mut self, t: &SyntaxToken) -> Option<Vec<Pitch>> {
        debug_assert!(t.kind().is_identifier());
        let ident = t.text().to_string();

        if let Some(chain) = self.macros.alias_macros.get(ident.as_str()) {
            if chain.is_empty() {
                self.error(
                    format!(
                        "Identifier in pitch chain cannot resolve to an empty alias macro: {}",
                        ident
                    ),
                    t.text_range(),
                );
                return None;
            }
            return Some(chain.clone());
        }

        if self.macros.simple_macros.contains_key(ident.as_str()) {
            self.error(
                format!(
                    "Identifier in pitch chain must resolve to an alias macro: {}",
                    ident
                ),
                t.text_range(),
            );
            return None;
        }

        if self.macros.complex_macros.contains_key(ident.as_str()) {
            self.error(
                format!(
                    "Identifier in pitch chain cannot resolve to a complex macro: {}",
                    ident
                ),
                t.text_range(),
            );
            return None;
        }

        self.error(
            format!("Undefined identifier in pitch chain: {}", ident),
            t.text_range(),
        );
        None
    }

    fn parse_pitch_chain_tokens(
        &mut self,
        tokens: &[SyntaxToken],
        allow_formal_single: bool,
        range: TextRange,
    ) -> Option<Note> {
        if tokens.is_empty() {
            return None;
        }

        let mut pitch_atoms: Vec<(Pitch, TextRange)> = Vec::new();
        let mut expect_pitch = true;
        let mut has_chain = false;

        for token in tokens {
            if expect_pitch {
                if token.kind().is_pitch() || token.kind().is_formal_pitch() {
                    let pitch = self.parse_pitch_atom(token, allow_formal_single)?;
                    pitch_atoms.push((pitch, token.text_range()));
                    expect_pitch = false;
                } else if token.kind().is_identifier() {
                    let chain = self.parse_pitch_chain_ident_as_chain(token)?;
                    for pitch in chain {
                        pitch_atoms.push((pitch, token.text_range()));
                    }
                    expect_pitch = false;
                } else {
                    self.error(
                        format!("Expected pitch token, got: {}", token.text()),
                        token.text_range(),
                    );
                    return None;
                }
            } else if token.kind().is_at() {
                has_chain = true;
                expect_pitch = true;
            } else if token.kind().is_plus() {
                has_chain = true;
                pitch_atoms.push((Pitch::Ratio(Rational32::new(2, 1)), token.text_range()));
            } else if token.kind().is_pitch_sustain() {
                has_chain = true;
                pitch_atoms.push((Pitch::Ratio(Rational32::new(1, 2)), token.text_range()));
            } else {
                self.error(
                    format!("Expected '@' in pitch chain, got: {}", token.text()),
                    token.text_range(),
                );
                return None;
            }
        }

        if expect_pitch {
            self.error("Pitch chain cannot end with '@'".to_string(), range);
            return None;
        }

        if has_chain
            && pitch_atoms
                .iter()
                .any(|(p, _)| matches!(p, Pitch::Rest | Pitch::Sustain))
        {
            self.error(
                "rest/sustain cannot be used inside pitch chain".to_string(),
                range,
            );
            return None;
        }

        if pitch_atoms.len() == 1 {
            return Some(Note::from_pitch(pitch_atoms[0].0, &self.state).with_pitch_chain(vec![
                pitch_atoms[0].0,
            ]));
        }

        let right = *pitch_atoms.last().expect("non-empty pitch atoms");
        let mut current_note = Note::from_pitch(right.0, &self.state);
        let mut current_base = (
            Note::base_note_from_pitch(
                right.0,
                current_note.freq,
                (self.state.base_note, self.state.base_frequency),
            ),
            current_note.freq,
        );

        for (pitch, _) in pitch_atoms[..pitch_atoms.len() - 1].iter().rev() {
            current_note = Note::note_from_pitch_with_base(*pitch, current_base.0, current_base.1);
            current_base = (
                Note::base_note_from_pitch(*pitch, current_note.freq, current_base),
                current_note.freq,
            );
        }

        Some(current_note.with_pitch_chain(
            pitch_atoms.iter().map(|(p, _)| *p).collect(),
        ))
    }

    fn parse_macro_invoke_tail_tokens(
        &mut self,
        tokens: &[SyntaxToken],
        range: TextRange,
    ) -> Option<Vec<Pitch>> {
        if tokens.is_empty() {
            return None;
        }

        let mut pitch_atoms: Vec<Pitch> = Vec::new();
        let mut expect_pitch = false;

        for token in tokens {
            if expect_pitch {
                if token.kind().is_pitch() || token.kind().is_formal_pitch() {
                    let pitch = self.parse_pitch_atom(token, false)?;
                    pitch_atoms.push(pitch);
                    expect_pitch = false;
                } else if token.kind().is_identifier() {
                    let chain = self.parse_pitch_chain_ident_as_chain(token)?;
                    pitch_atoms.extend(chain);
                    expect_pitch = false;
                } else {
                    self.error(
                        format!("Expected pitch token after '@', got: {}", token.text()),
                        token.text_range(),
                    );
                    return None;
                }
            } else if pitch_atoms.is_empty() && token.kind().is_pitch() {
                let pitch = self.parse_pitch_atom(token, false)?;
                pitch_atoms.push(pitch);
            } else if pitch_atoms.is_empty() && token.kind().is_identifier() {
                let chain = self.parse_pitch_chain_ident_as_chain(token)?;
                pitch_atoms.extend(chain);
            } else if token.kind().is_at() {
                expect_pitch = true;
            } else if token.kind().is_plus() {
                pitch_atoms.push(Pitch::Ratio(Rational32::new(2, 1)));
            } else if token.kind().is_pitch_sustain() {
                pitch_atoms.push(Pitch::Ratio(Rational32::new(1, 2)));
            } else {
                self.error(
                    format!(
                        "Expected '+', '-', or '@' after macro invoke, got: {}",
                        token.text()
                    ),
                    token.text_range(),
                );
                return None;
            }
        }

        if expect_pitch {
            self.error("Pitch chain cannot end with '@'".to_string(), range);
            return None;
        }

        if pitch_atoms.is_empty() {
            None
        } else {
            Some(pitch_atoms)
        }
    }

    fn eval_pitch_chain_pitches(&mut self, pitch_atoms: &[Pitch], range: TextRange) -> Option<Note> {
        if pitch_atoms.is_empty() {
            return None;
        }
        if pitch_atoms.len() > 1
            && pitch_atoms
                .iter()
                .any(|p| matches!(p, Pitch::Rest | Pitch::Sustain))
        {
            self.error(
                "rest/sustain cannot be used inside pitch chain".to_string(),
                range,
            );
            return None;
        }
        if pitch_atoms.len() == 1 {
            return Some(Note::from_pitch(pitch_atoms[0], &self.state).with_pitch_chain(vec![
                pitch_atoms[0],
            ]));
        }

        let right = *pitch_atoms.last().expect("non-empty pitch atoms");
        let mut current_note = Note::from_pitch(right, &self.state);
        let mut current_base = (
            Note::base_note_from_pitch(
                right,
                current_note.freq,
                (self.state.base_note, self.state.base_frequency),
            ),
            current_note.freq,
        );

        for pitch in pitch_atoms[..pitch_atoms.len() - 1].iter().rev() {
            current_note = Note::note_from_pitch_with_base(*pitch, current_base.0, current_base.1);
            current_base = (
                Note::base_note_from_pitch(*pitch, current_note.freq, current_base),
                current_note.freq,
            );
        }

        Some(current_note.with_pitch_chain(pitch_atoms.to_vec()))
    }

    fn compile_note_group(&mut self, n: &SyntaxNode) {
        debug_assert!(n.kind().is_node_note_group() || n.kind().is_node_note());
        let tokens = if n.kind().is_node_note_group() {
            n.children_with_tokens().collect()
        } else {
            vec![NodeOrToken::Node(n.clone())]
        };
        // Count sub-groups separated by semicolons
        let sub_group_count = tokens
            .iter()
            .filter(|nt| nt.as_token().is_some_and(|t| t.kind().is_semicolon()))
            .count()
            + 1;
        let mut cur_sub_group: Vec<CompileEvent> = Vec::new();
        // temporarily set quantize to sub-group duration
        self.state.quantize =
            self.state.quantize / Rational32::from_integer(sub_group_count as i32);

        for nt in tokens {
            match nt {
                NodeOrToken::Node(n) => match n.kind() {
                    SyntaxKind::NODE_NOTE => {
                        if let Some(notes) = self.parse_note(&n) {
                            for note in notes.into_iter() {
                                cur_sub_group.push(CompileEvent {
                                    body: EventBody::Note(note),
                                    start_time: self.state.time.clone(),
                                    range: n.text_range(),
                                    range_invoked: None,
                                });
                            }
                        }
                    }
                    _ => {
                        self.error(
                            format!("Unexpected node in note group: {:?}", n.kind()),
                            n.text_range(),
                        );
                    }
                },
                NodeOrToken::Token(t) => match t.kind() {
                    SyntaxKind::Semicolon => self.submit_note_sub_group(&mut cur_sub_group),
                    SyntaxKind::Colon => {
                        // do nothing, just a separator
                    }
                    _ => self.error(
                        format!("Unexpected token in note group: {}", t.text()),
                        t.text_range(),
                    ),
                },
            }
        }
        // submit last sub-group
        self.submit_note_sub_group(&mut cur_sub_group);
        // restore quantize and timestamp
        self.state.quantize =
            self.state.quantize * Rational32::from_integer(sub_group_count as i32);
        self.state.time = self
            .state
            .time
            .add_duration(self.state.quantize.neg(), &self.state);
        // advance time if the last token is a commas duration
        if let Some(last_token) = n
            .descendants_with_tokens()
            .last()
            .and_then(|nt| nt.into_token())
        {
            if last_token.kind().is_duration_commas() {
                let count = self.parse_duration_commas(&last_token).unwrap_or(0);
                let advance_dur = self.state.quantize * Rational32::from_integer(count as i32);
                self.state.time = self.state.time.add_duration(advance_dur, &self.state);
            }
        }
    }

    fn submit_note_sub_group(&mut self, cur_sub_group: &mut Vec<CompileEvent>) {
        let mut cur_dur = self.state.quantize;
        for note in cur_sub_group.iter_mut().rev() {
            if let EventBody::Note(n) = &mut note.body {
                if n.duration.is_zero() {
                    n.set_duration(cur_dur, &self.state);
                } else {
                    cur_dur = n.duration;
                }
            }
        }

        for note in cur_sub_group.drain(..) {
            self.push_event(note.body, note.range);
        }

        self.state.time = self
            .state
            .time
            .add_duration(self.state.quantize, &self.state);
    }

    fn parse_note(&mut self, n: &SyntaxNode) -> Option<Vec<Note>> {
        debug_assert!(n.kind().is_node_note());
        let duration_token = n.find_child_token_by_fn(|t| {
            t.kind().is_duration_commas() || t.kind().is_duration_fraction()
        });
        let duration = duration_token
            .and_then(|t| {
                if t.kind().is_duration_commas() {
                    self.parse_duration_commas(&t)
                        .map(|c| self.state.quantize * Rational32::from_integer((c + 1) as i32))
                } else if t.kind().is_duration_fraction() {
                    self.parse_duration_fraction(&t)
                } else {
                    None
                }
            })
            .unwrap_or(Rational32::zero());
        let mut notes: Vec<Note> = Vec::new();

        if let Some(node) = n
            .descendants()
            .find(|child| child.kind().is_node_macro_invoke())
        {
            match node.kind() {
                // Compile macro invoke
                SyntaxKind::NODE_MACRO_INVOKE => {
                    let ident = node
                        .find_child_token_by_fn(|t| t.kind().is_identifier())
                        .expect("Macro invoke node must have an identifier token")
                        .text()
                        .to_string();
                    let mut arg_chain_tokens: Vec<SyntaxToken> = node
                        .children_with_tokens()
                        .filter_map(|nt| nt.into_token())
                        .filter(|t| {
                            t.kind().is_pitch()
                                || t.kind().is_formal_pitch()
                                || t.kind().is_identifier()
                                || t.kind().is_at()
                                || t.kind().is_plus()
                        })
                        .collect();
                    if arg_chain_tokens
                        .first()
                        .is_some_and(|token| token.kind().is_identifier())
                    {
                        arg_chain_tokens.remove(0);
                    }
                    if arg_chain_tokens
                        .first()
                        .is_some_and(|token| token.kind().is_at())
                    {
                        arg_chain_tokens.remove(0);
                    }
                    let anchor_pitch_chain =
                        self.parse_macro_invoke_tail_tokens(&arg_chain_tokens, node.text_range());
                    if let Some(macro_notes) =
                        self.macros.simple_macros.get(ident.as_str()).cloned()
                    {
                        // !!!Simple macro invoke!!!
                        for mut note in macro_notes {
                            if let Some(anchor_chain) = &anchor_pitch_chain {
                                if !note.is_rest() && !note.is_sustain() {
                                    note.pitch_chain.extend(anchor_chain.iter().copied());
                                }
                            }
                            if let Some(note_live) =
                                self.eval_pitch_chain_pitches(&note.pitch_chain, node.text_range())
                            {
                                note.freq = note_live.freq;
                                note.pitch_ratio = note_live.pitch_ratio;
                            }
                            note.duration = duration;
                            note.duration_seconds = TimeStamp::dur_in_sec(duration, &self.state);
                            notes.push(note);
                        }
                    } else if let Some(alias_chain) =
                        self.macros.alias_macros.get(ident.as_str()).cloned()
                    {
                        if let Some(mut note) =
                            self.eval_pitch_chain_pitches(alias_chain.as_slice(), node.text_range())
                        {
                            if let Some(anchor_chain) = &anchor_pitch_chain {
                                if !note.is_rest() && !note.is_sustain() {
                                    note.pitch_chain.extend(anchor_chain.iter().copied());
                                }
                            }
                            if let Some(note_live) =
                                self.eval_pitch_chain_pitches(&note.pitch_chain, node.text_range())
                            {
                                note.freq = note_live.freq;
                                note.pitch_ratio = note_live.pitch_ratio;
                            }
                            note.duration = duration;
                            note.duration_seconds = TimeStamp::dur_in_sec(duration, &self.state);
                            notes.push(note);
                        }
                    } else if let Some(macro_events) =
                        self.macros.complex_macros.get(ident.as_str()).cloned()
                    {
                        // !!!Complex macro invoke!!!
                        // Directly push events and return empty notes
                        for e in macro_events {
                            if let EventBody::Note(mut note) = e.body {
                                if let Some(anchor_chain) = &anchor_pitch_chain {
                                    if !note.is_rest() && !note.is_sustain() {
                                        note.pitch_chain.extend(anchor_chain.iter().copied());
                                    }
                                }
                                if let Some(note_live) =
                                    self.eval_pitch_chain_pitches(&note.pitch_chain, n.text_range())
                                {
                                    note.freq = note_live.freq;
                                    note.pitch_ratio = note_live.pitch_ratio;
                                }
                                let start_time = TimeStamp {
                                    seconds: self.state.time.seconds + e.start_time.seconds,
                                    bars: self.state.time.bars + e.start_time.bars,
                                    ticks: self.state.time.ticks + e.start_time.ticks,
                                };
                                let ev = CompileEvent {
                                    body: EventBody::Note(note),
                                    start_time: start_time,
                                    range_invoked: Some(n.text_range()),
                                    ..e
                                };
                                self.events.push(ev);
                            }
                        }
                    } else {
                        self.error(
                            format!("Undefined macro invoked: {}", ident),
                            node.text_range(),
                        );
                    }
                }
                _ => {
                    self.error(
                        format!("Unexpected node in note: {:?}", node.kind()),
                        node.text_range(),
                    );
                }
            }
        } else {
            let Some(chain_node) = n
                .children()
                .find(|child| child.kind().is_node_pitch_chain())
            else {
                self.error(
                    "Note must have a pitch chain node".to_string(),
                    n.text_range(),
                );
                return None;
            };
            let chain_tokens: Vec<SyntaxToken> = chain_node
                .descendants_with_tokens()
                .filter_map(|nt| nt.into_token())
                .filter(|t| {
                    t.kind().is_pitch()
                        || t.kind().is_formal_pitch()
                        || t.kind().is_identifier()
                        || t.kind().is_at()
                        || t.kind().is_plus()
                })
                .collect();
            if chain_tokens.is_empty() {
                self.error(
                    "Note must have a pitch token or macro invoke node".to_string(),
                    chain_node.text_range(),
                );
                return None;
            }
            if let Some(mut note) =
                self.parse_pitch_chain_tokens(&chain_tokens, true, chain_node.text_range())
            {
                note.set_duration(duration, &self.state);
                notes.push(note);
            }
        }
        Some(notes)
    }

    fn finalize_negative_duration_notes(&mut self) {
        for event in self.events.iter_mut() {
            if let EventBody::Note(note) = &mut event.body {
                if note.duration.numer() < &0 {
                    let dur = note.duration.neg();
                    note.set_duration(dur, &self.state);
                    // adjust start time
                    event.start_time = event.start_time.add_duration(dur.neg(), &self.state);
                }
            }
        }
    }

    fn finalize_sustain_notes(&mut self) {
        let tolerance = 1e-4;
        let bucket_size = tolerance;
        let to_bucket = |sec: f64| -> i64 { (sec / bucket_size).round() as i64 };

        let mut sustain_infos = Vec::new();
        let mut end_buckets: HashMap<i64, Vec<usize>> = HashMap::new();

        for (idx, event) in self.events.iter().enumerate() {
            if let EventBody::Note(note) = &event.body {
                if note.is_sustain() {
                    sustain_infos.push((
                        event.start_time.seconds,
                        note.duration_seconds,
                        note.duration,
                        event.range,
                    ));
                } else {
                    let end = event.start_time.seconds + note.duration_seconds;
                    end_buckets.entry(to_bucket(end)).or_default().push(idx);
                }
            }
        }

        for (sustain_start, sustain_dur_sec, sustain_dur, sustain_range) in sustain_infos {
            let mut matched = false;

            let center_bucket = to_bucket(sustain_start);
            let mut candidate_indices = Vec::new();
            for key in [center_bucket - 1, center_bucket, center_bucket + 1] {
                if let Some(indices) = end_buckets.get(&key) {
                    candidate_indices.extend(indices.iter().copied());
                }
            }

            for idx in candidate_indices {
                let old_end = match &self.events[idx].body {
                    EventBody::Note(note) if !note.is_sustain() => {
                        self.events[idx].start_time.seconds + note.duration_seconds
                    }
                    _ => continue,
                };

                if (old_end - sustain_start).abs() < tolerance {
                    if let EventBody::Note(note) = &mut self.events[idx].body {
                        note.duration += sustain_dur;
                        note.duration_seconds += sustain_dur_sec;
                    }

                    let new_end = old_end + sustain_dur_sec;
                    let old_bucket = to_bucket(old_end);
                    let new_bucket = to_bucket(new_end);
                    if old_bucket != new_bucket {
                        if let Some(indices) = end_buckets.get_mut(&old_bucket) {
                            if let Some(pos) = indices.iter().position(|&i| i == idx) {
                                indices.swap_remove(pos);
                            }
                        }
                        end_buckets.entry(new_bucket).or_default().push(idx);
                    }
                    matched = true;
                }
            }

            if !matched {
                self.warn(
                    "Sustain note has no matching preceding note".to_string(),
                    sustain_range,
                );
            }
        }
        self.events.retain(|e| {
            if let EventBody::Note(n) = &e.body {
                !n.is_sustain()
            } else {
                true
            }
        });
    }

    fn error(&mut self, message: String, span: TextRange) {
        self.diagnostics.push(Diagnostic {
            message,
            level: DiagnosticLevel::Error,
            span,
        });
    }

    fn warn(&mut self, message: String, span: TextRange) {
        self.diagnostics.push(Diagnostic {
            message,
            level: DiagnosticLevel::Warning,
            span,
        });
    }

    fn push_event(&mut self, body: EventBody, range: TextRange) {
        self.events.push(CompileEvent {
            body,
            range,
            range_invoked: None,
            start_time: self.state.time.clone(),
        });
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path, sync::Arc};

    use crate::rowan::parse_fn::parse_source;

    use super::*;

    fn get_span_text(range: &TextRange, source: &str) -> (usize, usize, String) {
        let start: usize = range.start().into();
        let end: usize = range.end().into();
        let text = source.get(start..end).unwrap_or("").to_string();
        (start, end, text)
    }

    fn compile_source(source: &str) -> Compiler {
        let parsed = parse_source(Arc::from(source));
        let mut compiler = Compiler::new();
        compiler.compile(&parsed.syntax_node());
        compiler
    }

    fn has_error_diagnostics(compiler: &Compiler) -> bool {
        compiler
            .diagnostics
            .iter()
            .any(|d| matches!(d.level, DiagnosticLevel::Error))
    }

    fn first_note_freq(compiler: &Compiler) -> f32 {
        compiler
            .events
            .iter()
            .find_map(|e| match &e.body {
                EventBody::Note(n) => Some(n.freq),
                _ => None,
            })
            .expect("expected one note event")
    }

    #[test]
    fn compile_pitch_chain_right_to_left() {
        let compiler = compile_source("C4@3/2,\n");
        assert!(!has_error_diagnostics(&compiler));

        let note = compiler
            .events
            .iter()
            .find_map(|e| match &e.body {
                EventBody::Note(n) => Some(n.clone()),
                _ => None,
            })
            .expect("expected one note event");

        let right_freq = 261.63f32 * 1.5;
        let expected = right_freq * 2f32.powf((60f32 - 67f32) / 12.0);
        assert!((note.freq - expected).abs() < 0.2);
    }

    #[test]
    fn compile_pitch_chain_rejects_rest_or_sustain() {
        let compiler = compile_source(".@C4,\n");
        assert!(compiler.diagnostics.iter().any(|d| {
            d.message
                .contains("rest/sustain cannot be used inside pitch chain")
        }));
    }

    #[test]
    fn compile_pitch_chain_identifier_tail_from_alias_macro_ok() {
        let compiler = compile_source("m = 3/2\nC4@m,\n");
        assert!(!has_error_diagnostics(&compiler));

        let note = compiler
            .events
            .iter()
            .find_map(|e| match &e.body {
                EventBody::Note(n) => Some(n.clone()),
                _ => None,
            })
            .expect("expected one note event");

        let right_freq = 261.63f32 * 1.5;
        let expected = right_freq * 2f32.powf((60f32 - 67f32) / 12.0);
        assert!((note.freq - expected).abs() < 0.3);
    }

    #[test]
    fn compile_pitch_chain_identifier_tail_from_multi_simple_macro_reports_error() {
        let compiler = compile_source("m = C4:D4\nC4@m,\n");
        assert!(compiler.diagnostics.iter().any(|d| {
            d.message
                .contains("Identifier in pitch chain must resolve to an alias macro")
        }));
    }

    #[test]
    fn compile_pitch_chain_identifier_tail_from_complex_macro_reports_error() {
        let compiler = compile_source("m =\nC4,\n\nC4@m,\n");
        assert!(compiler.diagnostics.iter().any(|d| {
            d.message
                .contains("Identifier in pitch chain cannot resolve to a complex macro")
        }));
    }

    #[test]
    fn compile_macro_invoke_head_unrestricted_tail_identifier_restricted() {
        let compiler = compile_source("m = C4:D4\nb = 3/2\nm@b,\n");
        assert!(!has_error_diagnostics(&compiler));

        let note_count = compiler
            .events
            .iter()
            .filter(|e| matches!(e.body, EventBody::Note(_)))
            .count();
        assert_eq!(note_count, 2);
    }

    #[test]
    fn compile_simple_macro_def_allows_pitch_chain_with_identifier() {
        let compiler = compile_source("a = 3/2\nm = C4@a\nm,\n");
        assert!(!has_error_diagnostics(&compiler));

        let note = compiler
            .events
            .iter()
            .find_map(|e| match &e.body {
                EventBody::Note(n) => Some(n.clone()),
                _ => None,
            })
            .expect("expected one note event");

        let right_freq = 261.63f32 * 1.5;
        let expected = right_freq * 2f32.powf((60f32 - 67f32) / 12.0);
        assert!((note.freq - expected).abs() < 0.3);
    }

    #[test]
    fn compile_simple_macro_anchor_pitch_chain() {
        let compiler = compile_source("m = 3/2\nm@D4,\n");
        assert!(!has_error_diagnostics(&compiler));

        let note = compiler
            .events
            .iter()
            .find_map(|e| match &e.body {
                EventBody::Note(n) => Some(n.clone()),
                _ => None,
            })
            .expect("expected one note event");

        let expected = 293.66f32 * 1.5;
        assert!((note.freq - expected).abs() < 0.3);
    }

    #[test]
    fn compile_complex_macro_anchor_pitch_chain() {
        let compiler = compile_source("m =\n3/2,\n\nm@D4,\n");
        assert!(!has_error_diagnostics(&compiler));

        let note = compiler
            .events
            .iter()
            .find_map(|e| match &e.body {
                EventBody::Note(n) => Some(n.clone()),
                _ => None,
            })
            .expect("expected one note event");

        let expected = 293.66f32 * 1.5;
        assert!((note.freq - expected).abs() < 0.3);
    }

    #[test]
    fn compile_simple_macro_anchor_appends_pitch_chain() {
        let direct = compile_source("3/2@D4,\n");
        let from_macro = compile_source("m = 3/2\nm@D4,\n");
        assert!(!has_error_diagnostics(&direct));
        assert!(!has_error_diagnostics(&from_macro));

        let direct_freq = first_note_freq(&direct);
        let macro_freq = first_note_freq(&from_macro);
        assert!((direct_freq - macro_freq).abs() < 1e-3);
    }

    #[test]
    fn compile_complex_macro_anchor_appends_pitch_chain() {
        let direct = compile_source("3/2@D4,\n");
        let from_macro = compile_source("m =\n3/2,\n\nm@D4,\n");
        assert!(!has_error_diagnostics(&direct));
        assert!(!has_error_diagnostics(&from_macro));

        let direct_freq = first_note_freq(&direct);
        let macro_freq = first_note_freq(&from_macro);
        assert!((direct_freq - macro_freq).abs() < 1e-3);
    }

    #[test]
    fn compile_pitch_chain_plus_suffix_equivalent_to_ratio_up() {
        let with_suffix = compile_source("3/2+,\n");
        let with_ratio = compile_source("3/2@2/1,\n");
        assert!(!has_error_diagnostics(&with_suffix));
        assert!(!has_error_diagnostics(&with_ratio));

        let suffix_freq = first_note_freq(&with_suffix);
        let ratio_freq = first_note_freq(&with_ratio);
        assert!((suffix_freq - ratio_freq).abs() < 1e-3);
    }

    #[test]
    fn compile_pitch_chain_minus_suffix_equivalent_to_ratio_down() {
        let with_suffix = compile_source("3/2-,\n");
        let with_ratio = compile_source("3/2@1/2,\n");
        assert!(!has_error_diagnostics(&with_suffix));
        assert!(!has_error_diagnostics(&with_ratio));

        let suffix_freq = first_note_freq(&with_suffix);
        let ratio_freq = first_note_freq(&with_ratio);
        assert!((suffix_freq - ratio_freq).abs() < 1e-3);
    }

    #[test]
    fn compile_macro_invoke_plus_suffix_equivalent_to_ratio_up() {
        let with_suffix = compile_source("m = 3/2\nm+,\n");
        let with_ratio = compile_source("m = 3/2\nm@2/1,\n");
        assert!(!has_error_diagnostics(&with_suffix));
        assert!(!has_error_diagnostics(&with_ratio));

        let suffix_freq = first_note_freq(&with_suffix);
        let ratio_freq = first_note_freq(&with_ratio);
        assert!((suffix_freq - ratio_freq).abs() < 1e-3);
    }

    #[test]
    fn compile_macro_invoke_minus_suffix_equivalent_to_ratio_down() {
        let with_suffix = compile_source("m = 3/2\nm-,\n");
        let with_ratio = compile_source("m = 3/2\nm@1/2,\n");
        assert!(!has_error_diagnostics(&with_suffix));
        assert!(!has_error_diagnostics(&with_ratio));

        let suffix_freq = first_note_freq(&with_suffix);
        let ratio_freq = first_note_freq(&with_ratio);
        assert!((suffix_freq - ratio_freq).abs() < 1e-3);
    }

    #[test]
    fn compile_simple_macro_preserves_pitch_chain_semantics() {
        let direct = compile_source("4/5@3/2,\n");
        let from_macro = compile_source("m = 4/5@3/2\nm,\n");
        assert!(!has_error_diagnostics(&direct));
        assert!(!has_error_diagnostics(&from_macro));

        let direct_freq = first_note_freq(&direct);
        let macro_freq = first_note_freq(&from_macro);
        assert!((direct_freq - macro_freq).abs() < 1e-3);
    }

    #[test]
    fn compile_complex_macro_preserves_pitch_chain_semantics() {
        let direct = compile_source("4/5@3/2,\n");
        let from_macro = compile_source("m =\n4/5@3/2,\n\nm,\n");
        assert!(!has_error_diagnostics(&direct));
        assert!(!has_error_diagnostics(&from_macro));

        let direct_freq = first_note_freq(&direct);
        let macro_freq = first_note_freq(&from_macro);
        assert!((direct_freq - macro_freq).abs() < 1e-3);
    }

    #[test]
    fn compile_base_pitch_accepts_non_frequency_reference() {
        let compiler = compile_source("<C4=3/2>\n");
        assert!(!has_error_diagnostics(&compiler));
        assert!(
            compiler
                .events
                .iter()
                .any(|e| matches!(e.body, EventBody::BaseNoteDef(_)))
        );
        assert!(
            compiler
                .events
                .iter()
                .any(|e| matches!(e.body, EventBody::BaseFequencyDef(_)))
        );
    }

    #[test]
    fn compile_base_pitch_rhs_identifier_chain_alias_macro_ok() {
        let direct = compile_source("<C4=3/2@5/4>\n");
        let from_ident = compile_source("a = 3/2@5/4\n<C4=a>\n");
        assert!(!has_error_diagnostics(&direct));
        assert!(!has_error_diagnostics(&from_ident));

        let direct_freq = direct
            .events
            .iter()
            .find_map(|e| match e.body {
                EventBody::BaseFequencyDef(f) => Some(f),
                _ => None,
            })
            .expect("expected base frequency event");
        let ident_freq = from_ident
            .events
            .iter()
            .find_map(|e| match e.body {
                EventBody::BaseFequencyDef(f) => Some(f),
                _ => None,
            })
            .expect("expected base frequency event");

        assert!((direct_freq - ident_freq).abs() < 1e-3);
    }

    #[test]
    fn compile_base_pitch_rhs_identifier_chain_rejects_multi_note_simple_macro() {
        let compiler = compile_source("a = C4:D4\n<C4=a>\n");
        assert!(compiler.diagnostics.iter().any(|d| {
            d.message
                .contains("Identifier in base pitch RHS must resolve to an alias macro")
        }));
    }

    #[test]
    fn compile_base_pitch_spell_without_rhs_infers_frequency() {
        let shorthand = compile_source("<D4>\n");
        let explicit = compile_source("<D4=D4>\n");
        assert!(!has_error_diagnostics(&shorthand));
        assert!(!has_error_diagnostics(&explicit));

        let shorthand_freq = shorthand
            .events
            .iter()
            .find_map(|e| match e.body {
                EventBody::BaseFequencyDef(f) => Some(f),
                _ => None,
            })
            .expect("expected base frequency event");
        let explicit_freq = explicit
            .events
            .iter()
            .find_map(|e| match e.body {
                EventBody::BaseFequencyDef(f) => Some(f),
                _ => None,
            })
            .expect("expected base frequency event");

        assert!((shorthand_freq - explicit_freq).abs() < 1e-3);
    }

    #[test]
    fn dump_sample_compilation() {
        let path = Path::new("src/tests/sample.symi");
        let source: Arc<str> =
            Arc::from(fs::read_to_string(path).expect("failed to read tests/sample.symi"));
        let result = parse_source(source.clone());

        let root = result.syntax_node();
        let mut compiler = Compiler::new();
        compiler.compile(&root);

        let mut output = String::new();

        // Format events
        output.push_str("=== COMPILATION EVENTS ===\n\n");
        output.push_str(
            "source,event,event_arg,freq,start_sec,start_bar,start_tick,dur_sec,dur_tick\n",
        );
        for (idx, event) in compiler.events.iter().enumerate() {
            let (start, end, text) = get_span_text(&event.range, &source);
            match event {
                CompileEvent {
                    body: EventBody::Note(note),
                    ..
                } => {
                    output.push_str(&format!(
                        "{},{},{},{:.3},{:.3},{},{},{:.3},{}\n",
                        idx,
                        "Note",
                        format!("\"[{}, {}] {}\"", start, end, text.replace('\n', "\\n")),
                        note.freq,
                        event.start_time.seconds,
                        event.start_time.bars,
                        event.start_time.ticks,
                        note.duration_seconds,
                        note.duration,
                    ));
                }
                CompileEvent {
                    body: EventBody::BaseNoteDef(pitch_spell),
                    ..
                } => {
                    output.push_str(&format!(
                        "{},{},{},\"{:?}\",,,,\n",
                        idx,
                        "BaseNoteDef",
                        format!("\"[{}, {}] {}\"", start, end, text.replace('\n', "\\n")),
                        pitch_spell,
                    ));
                }
                _ => {
                    output.push_str(&format!(
                        "{},{},{},,,,\n",
                        idx,
                        "OtherEvent",
                        format!("\"[{}, {}] {}\"", start, end, text.replace('\n', "\\n")),
                    ));
                }
            }
        }

        // Format diagnostics
        output.push_str("\n=== DIAGNOSTICS ===\n\n");
        for diag in &compiler.diagnostics {
            let (start, end, text) = get_span_text(&diag.span, &source);
            output.push_str(&format!(
                "[{:?}] {} at [{}, {}]\n  Source: {:?}\n",
                diag.level, diag.message, start, end, text
            ));
        }

        // Format macros
        output.push_str("\n=== MACROS ===\n\n");
        output.push_str("Simple Macros:\n");
        for (name, notes) in &compiler.macros.simple_macros {
            output.push_str(&format!("  {} -> {:?}\n", name, notes));
        }
        output.push_str("\nComplex Macros:\n");
        for (name, events) in &compiler.macros.complex_macros {
            output.push_str(&format!("  {} -> {} events\n", name, events.len()));
        }

        let out_path = path.with_file_name("sample_compiled.txt");
        fs::write(out_path, output).expect("failed to write tests/sample_compiled.txt");
    }
}
