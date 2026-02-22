use std::{
    cell::RefCell,
    mem::{replace, swap, take},
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
            node.kind().is_node_macrodef_simple()
                || node.kind().is_node_macrodef_complex()
        );
        let ident_tok = node
            .find_child_token_by_fn(|t| t.kind().is_identifier())
            .expect("Macro definition must have an identifier token");
        let note_kind = node.kind();
        match note_kind {
            SyntaxKind::NODE_MACRODEF_SIMPLE => {
                let mut pitches: Vec<Note> = Vec::new();
                let mut current_tokens: Vec<SyntaxToken> = Vec::new();
                for nt in node.children_with_tokens() {
                    let Some(token) = nt.into_token() else {
                        continue;
                    };
                    match token.kind() {
                        kind if kind.is_pitch() || kind.is_at() => current_tokens.push(token),
                        SyntaxKind::Colon => {
                            if !current_tokens.is_empty() {
                                if let Some(note) = self.parse_pitch_chain_tokens(
                                    &current_tokens,
                                    false,
                                    node.text_range(),
                                ) {
                                    pitches.push(note);
                                }
                                current_tokens.clear();
                            }
                        }
                        _ => {}
                    }
                }
                if !current_tokens.is_empty() {
                    if let Some(note) =
                        self.parse_pitch_chain_tokens(&current_tokens, false, node.text_range())
                    {
                        pitches.push(note);
                    }
                }
                self.macros
                    .simple_macros
                    .insert(ident_tok.text().to_string(), pitches);
            }
            SyntaxKind::NODE_MACRODEF_COMPLEX => {
                let mut state = CompileState {
                    time: TimeStamp {
                        seconds: 0.0,
                        bars: 0,
                        ticks: Rational32::new(0, *self.state.quantize.denom()),
                    },
                    ..self.state
                };
                swap(&mut self.state, &mut state);
                let events = take(&mut self.events);
                let node_body = node
                    .find_child_node_by_fn(|n| n.kind().is_node_macrodef_complex_body())
                    .expect("Macro complex definition must have a body node");
                for node in node_body.children() {
                    self.compile_normal_line(&node);
                    self.reset_ticks();
                }
                self.macros.complex_macros.insert(
                    ident_tok.text().to_string(),
                    replace(&mut self.events, events),
                );
                self.state = state;
                self.events = take(&mut self.events);
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
        let pitch_tokens: Vec<SyntaxToken> = n
            .children_with_tokens()
            .filter_map(|nt| nt.into_token())
            .filter(|t| t.kind().is_pitch())
            .collect();

        if pitch_tokens.is_empty() {
            self.error(
                "Base pitch definition must contain at least one pitch token".to_string(),
                n.text_range(),
            );
            return;
        }
        if pitch_tokens.len() > 2 {
            self.error(
                "Base pitch definition can contain at most two pitch tokens".to_string(),
                n.text_range(),
            );
            return;
        }

        let first = pitch_tokens[0].clone();
        let second = pitch_tokens.get(1).cloned();

        let pitch_spell =
            if first.kind().is_pitch_spell_octave() || first.kind().is_pitch_spell_simple() {
                self.parse_pitch(&first, false)
            } else {
                None
            };

        let pitch_ref = if let Some(second) = second {
            self.parse_pitch(&second, false)
        } else if pitch_spell.is_none() {
            self.parse_pitch(&first, false)
        } else {
            None
        };

        if let Some(spell) = pitch_spell {
            self.state.base_note = match spell.pitch {
                Pitch::SpellOctave(s) => s,
                Pitch::SpellSimple(s) => s,
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
                    "Base pitch definition must have either a pitch spell or pitch reference"
                        .to_string(),
                    n.text_range(),
                );
            }
        }
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
            return Some(Note::from_pitch(pitch_atoms[0].0, &self.state));
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

        Some(current_note)
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
        let cur_sub_group: RefCell<Vec<CompileEvent>> = RefCell::default();
        // temporarily set quantize to sub-group duration
        self.state.quantize =
            self.state.quantize / Rational32::from_integer(sub_group_count as i32);
        // closure to submit current sub-group
        let submit_note_group = |mut _self: &mut Compiler| {
            // fill durations for notes in current sub-group
            let mut cur_dur = _self.state.quantize;
            for note in cur_sub_group.borrow_mut().iter_mut().rev() {
                if let EventBody::Note(n) = &mut note.body {
                    if n.duration.is_zero() {
                        n.set_duration(cur_dur, &_self.state);
                    } else {
                        cur_dur = n.duration;
                    }
                }
            }
            // push notes to events
            for note in cur_sub_group.borrow_mut().drain(..) {
                _self.push_event(note.body, note.range);
            }
            // advance time by quantize
            _self.state.time = _self
                .state
                .time
                .add_duration(_self.state.quantize, &_self.state);
        };

        for nt in tokens {
            match nt {
                NodeOrToken::Node(n) => match n.kind() {
                    SyntaxKind::NODE_NOTE => {
                        if let Some(notes) = self.parse_note(&n) {
                            for note in notes.into_iter() {
                                cur_sub_group.borrow_mut().push(CompileEvent {
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
                    SyntaxKind::Semicolon => submit_note_group(self),
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
        submit_note_group(self);
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
            .children()
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
                            t.kind().is_pitch() || t.kind().is_formal_pitch() || t.kind().is_at()
                        })
                        .collect();
                    if arg_chain_tokens
                        .first()
                        .is_some_and(|token| token.kind().is_at())
                    {
                        arg_chain_tokens.remove(0);
                    }
                    let arg_pitch_tokens = self.parse_pitch_chain_tokens(
                        &arg_chain_tokens,
                        false,
                        node.text_range(),
                    );
                    let mut new_base_pitch = arg_pitch_tokens
                        .as_ref()
                        .map(|n| (freq2spell(n.freq, &self.state), n.freq));
                    if let Some((anchor_base_note, anchor_base_freq)) = new_base_pitch {
                        let old_base_pitch = (self.state.base_note, self.state.base_frequency);
                        self.state.base_note = anchor_base_note;
                        self.state.base_frequency = anchor_base_freq;
                        new_base_pitch = Some(old_base_pitch);
                    }
                    if let Some(macro_notes) = self.macros.simple_macros.get(ident.as_str()) {
                        // !!!Simple macro invoke!!!
                        for mut note in macro_notes.to_vec() {
                            let note_live = Note::from_pitch(note.pitch, &self.state);
                            note.freq = note_live.freq;
                            note.pitch_ratio = note_live.pitch_ratio;
                            note.duration = duration;
                            note.duration_seconds = TimeStamp::dur_in_sec(duration, &self.state);
                            notes.push(note);
                        }
                    } else if let Some(macro_events) =
                        self.macros.complex_macros.get(ident.as_str())
                    {
                        // !!!Complex macro invoke!!!
                        // Directly push events and return empty notes
                        for e in macro_events.to_vec().into_iter() {
                            if let EventBody::Note(mut note) = e.body {
                                note.freq = Note::from_pitch(note.pitch.clone(), &self.state).freq;
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
                    if let Some((old_base_note, old_base_freq)) = new_base_pitch {
                        self.state.base_note = old_base_note;
                        self.state.base_frequency = old_base_freq;
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
            let chain_tokens: Vec<SyntaxToken> = n
                .children_with_tokens()
                .filter_map(|nt| nt.into_token())
                .filter(|t| t.kind().is_pitch() || t.kind().is_formal_pitch() || t.kind().is_at())
                .collect();
            if chain_tokens.is_empty() {
                self.error(
                    "Note must have a pitch token or macro invoke node".to_string(),
                    n.text_range(),
                );
                return None;
            }
            if let Some(mut note) =
                self.parse_pitch_chain_tokens(&chain_tokens, true, n.text_range())
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
        // 不对 events 排序：直接在全量事件中匹配 sustain
        // 对每个 sustain note：寻找“结束时间 == sustain.start_time”的候选音符
        // 在允许误差 1e-4 内选择最接近者（并优先选择 start_time 更晚的音符）
        // 然后将 sustain 的 duration 加到该音符上
        let tolerance = 1e-4;
        let mut sustain_infos = Vec::new();
        for event in &self.events {
            if let EventBody::Note(note) = &event.body {
                if note.pitch == Pitch::Sustain {
                    sustain_infos.push((
                        event.start_time.seconds,
                        note.duration_seconds,
                        note.duration,
                        event.range,
                    ));
                }
            }
        }

        for (sustain_start, sustain_dur_sec, sustain_dur, sustain_range) in sustain_infos {
            let mut matched = false;
            for event in self.events.iter_mut() {
                if let EventBody::Note(note) = &mut event.body {
                    if note.pitch == Pitch::Sustain {
                        continue;
                    }
                    let end = event.start_time.seconds + note.duration_seconds;
                    if (end - sustain_start).abs() < tolerance {
                        note.duration += sustain_dur;
                        note.duration_seconds += sustain_dur_sec;
                        matched = true;
                    }
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
                n.pitch != Pitch::Sustain
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

    #[test]
    fn compile_pitch_chain_right_to_left() {
        let compiler = compile_source("C4@3/2,\n");
        assert!(!has_error_diagnostics(&compiler));

        let note = compiler
            .events
            .iter()
            .find_map(|e| match &e.body {
                EventBody::Note(n) => Some(*n),
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
    fn compile_macro_arg_pitch_chain_reports_error() {
        let compiler = compile_source("m = 3/2\nm(C4@3/2),\n");
        assert!(has_error_diagnostics(&compiler));
    }

    #[test]
    fn compile_simple_macro_anchor_pitch_chain() {
        let compiler = compile_source("m = 3/2\nm@D4,\n");
        assert!(!has_error_diagnostics(&compiler));

        let note = compiler
            .events
            .iter()
            .find_map(|e| match &e.body {
                EventBody::Note(n) => Some(*n),
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
                EventBody::Note(n) => Some(*n),
                _ => None,
            })
            .expect("expected one note event");

        let expected = 293.66f32 * 1.5;
        assert!((note.freq - expected).abs() < 0.3);
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
