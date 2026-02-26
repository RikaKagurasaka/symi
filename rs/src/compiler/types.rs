use std::collections::HashMap;

use super::rational::Rational32;
use regex::Regex;
use rowan::TextRange;
use strum::Display;

pub type PitchSpell = i16; // note: 0=C-1, 1=C#-1, ..., 60=C4, ... 
pub type PitchChain = Vec<Pitch>;

#[derive(Debug, Display, Clone, Copy, PartialEq)]
pub enum Pitch {
    SpellOctave(PitchSpell),
    SpellSimple(PitchSpell),
    Frequency(f32),
    Ratio(Rational32),
    Edo(Rational32),
    Cents(i32),
    Rest,
    Sustain,
}
fn char_to_semitone(c: char) -> Option<i16> {
    match c {
        'C' => Some(0),
        'D' => Some(2),
        'E' => Some(4),
        'F' => Some(5),
        'G' => Some(7),
        'A' => Some(9),
        'B' => Some(11),
        _ => None,
    }
}

impl Pitch {
    pub fn parse_spell_octave(s: &str) -> Option<Self> {
        let regex = Regex::new(r"^([A-G])([#b]*)(-?\d+)$").unwrap();
        if let Some(caps) = regex.captures(s) {
            let base_char = caps.get(1)?.as_str().chars().next()?;
            let accidentals = caps.get(2)?.as_str();
            let octave_str = caps.get(3)?.as_str();

            let mut semitone = char_to_semitone(base_char)?;
            for acc in accidentals.chars() {
                match acc {
                    '#' => semitone += 1,
                    'b' => semitone -= 1,
                    _ => return None,
                }
            }
            let octave: i16 = octave_str.parse().ok()?;
            let pitch_spell = semitone + (octave + 1) * 12;
            Some(Pitch::SpellOctave(pitch_spell))
        } else {
            None
        }
    }

    pub fn parse_spell_simple(s: &str) -> Option<Self> {
        let regex = Regex::new(r"^([A-G])([#b]*)$").unwrap();
        if let Some(caps) = regex.captures(s) {
            let base_char = caps.get(1)?.as_str().chars().next()?;
            let accidentals = caps.get(2)?.as_str();

            let mut semitone = char_to_semitone(base_char)?;
            for acc in accidentals.chars() {
                match acc {
                    '#' => semitone += 1,
                    'b' => semitone -= 1,
                    _ => return None,
                }
            }
            Some(Pitch::SpellSimple(semitone))
        } else {
            None
        }
    }

    pub fn parse_fequency(s: &str) -> Option<Self> {
        s.parse::<f32>().ok().map(Pitch::Frequency)
    }

    pub fn parse_ratio(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() == 2 {
            let numerator = parts[0].parse::<i32>().ok()?;
            let denominator = parts[1].parse::<i32>().ok()?;
            Some(Pitch::Ratio(Rational32::new(numerator, denominator)))
        } else {
            None
        }
    }

    pub fn parse_edo(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('\\').collect();
        if parts.len() == 2 {
            let numerator = parts[0].parse::<i32>().ok()?;
            let denominator = parts[1].parse::<i32>().ok()?;
            Some(Pitch::Edo(Rational32::new(numerator, denominator)))
        } else {
            None
        }
    }

    pub fn parse_cents(s: &str) -> Option<Self> {
        s[..s.len() - 1].parse::<i32>().ok().map(Pitch::Cents)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeStamp {
    pub seconds: f64,
    pub bars: u32,
    pub ticks: Rational32,
}

impl Default for TimeStamp {
    fn default() -> Self {
        Self {
            seconds: 0.0,
            bars: 0,
            ticks: Rational32::new(0, 4),
        }
    }
}

impl TimeStamp {
    pub fn dur_in_sec(duration: Rational32, state: &CompileState) -> f64 {
        let full_notes = duration.to_f32().unwrap();
        let full_note_per_minute = state.bpm
            * (state.beat_duration)
                .to_f32()
                .expect("Rational32 to f32 conversion failed");
        (full_notes / full_note_per_minute as f32 * 60.0) as f64
    }

    pub fn add_duration(&self, duration: Rational32, state: &CompileState) -> Self {
        let mut _self = self.clone();
        // Update ticks
        _self.ticks += duration;
        // Update seconds
        _self.seconds += TimeStamp::dur_in_sec(duration, state);
        _self
    }

    pub fn reduct_to_quantize(&self, quantize: Rational32) -> Self {
        let mut _self = self.clone();
        _self.ticks = _self.ticks.reduct_to(*quantize.denom());
        _self
    }

    pub fn next_bar(&self, time_signature: Rational32) -> Self {
        let mut next = self.clone();
        next.bars += 1;
        next.ticks = Rational32::new(0, *time_signature.denom());
        next
    }

    pub fn is_zero(&self) -> bool {
        self.seconds == 0.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    pub pitch_chain: PitchChain,
    pub freq: f32,
    pub duration: Rational32,
    pub duration_seconds: f64,
    pub pitch_ratio: f32,
}
#[allow(unused)]
pub(crate) fn spell2freq(spell: i16, state: &CompileState) -> f32 {
    let semitone_diff = spell - state.base_note;
    state.base_frequency * 2f32.powf(semitone_diff as f32 / 12.0)
}

pub(crate) fn freq2spell(freq: f32, state: &CompileState) -> i16 {
    let semitone_diff = 12.0 * (freq / state.base_frequency).log2();
    (semitone_diff.round() as i16) + state.base_note
}

impl Note {
    pub fn from_pitch(pitch: Pitch, state: &CompileState) -> Self {
        let base_note = state.base_note;
        let base_frequency = state.base_frequency;
        let freq = match pitch {
            Pitch::SpellOctave(spell) => {
                let semitone_diff = spell - base_note;
                base_frequency * 2f32.powf(semitone_diff as f32 / 12.0)
            }
            Pitch::SpellSimple(spell) => {
                let semitone_diff = spell.div_euclid(12) * 12 + (spell - base_note).rem_euclid(12);
                base_frequency * 2f32.powf(semitone_diff as f32 / 12.0)
            }
            Pitch::Frequency(f) => f,
            Pitch::Ratio(r) => {
                base_frequency * r.to_f32().expect("Rational32 to f32 conversion failed")
            }
            Pitch::Edo(r) => {
                let semitone_diff = r.to_f32().expect("Rational32 to f32 conversion failed");
                base_frequency * 2f32.powf(semitone_diff)
            }
            Pitch::Cents(c) => base_frequency * 2f32.powf(c as f32 / 1200.0),
            Pitch::Rest | Pitch::Sustain => 0.0,
        };
        Self {
            pitch_chain: vec![pitch],
            freq,
            duration: Rational32::new(0, 4),
            duration_seconds: 0.0,
            pitch_ratio: freq / base_frequency,
        }
    }

    pub fn note_from_pitch_with_base(pitch: Pitch, base_note: i16, base_frequency: f32) -> Note {
        let freq = match pitch {
            Pitch::SpellOctave(spell) => {
                let semitone_diff = spell - base_note;
                base_frequency * 2f32.powf(semitone_diff as f32 / 12.0)
            }
            Pitch::SpellSimple(spell) => {
                let semitone_diff = spell - (base_note % 12);
                base_frequency * 2f32.powf(semitone_diff as f32 / 12.0)
            }
            Pitch::Frequency(f) => f,
            Pitch::Ratio(r) => {
                base_frequency * r.to_f32().expect("Rational32 to f32 conversion failed")
            }
            Pitch::Edo(r) => {
                let semitone_diff = r.to_f32().expect("Rational32 to f32 conversion failed");
                base_frequency * 2f32.powf(semitone_diff)
            }
            Pitch::Cents(c) => base_frequency * 2f32.powf(c as f32 / 1200.0),
            Pitch::Rest | Pitch::Sustain => 0.0,
        };
        Note {
            pitch_chain: vec![pitch],
            freq,
            duration: Rational32::new(0, 4),
            duration_seconds: 0.0,
            pitch_ratio: freq / base_frequency,
        }
    }

    pub fn base_note_from_pitch(pitch: Pitch, freq: f32, current_base: (i16, f32)) -> i16 {
        match pitch {
            Pitch::SpellOctave(s) | Pitch::SpellSimple(s) => s,
            _ => {
                let mut state = CompileState::new();
                state.base_note = current_base.0;
                state.base_frequency = current_base.1;
                freq2spell(freq, &state)
            }
        }
    }

    pub fn set_duration(&mut self, duration: Rational32, state: &CompileState) {
        self.duration = duration;
        self.duration_seconds = TimeStamp::dur_in_sec(duration, state);
    }

    pub fn with_pitch_chain(mut self, pitch_chain: PitchChain) -> Self {
        self.pitch_chain = pitch_chain;
        self
    }

    pub fn is_rest(&self) -> bool {
        self.pitch_chain.len() == 1 && matches!(self.pitch_chain[0], Pitch::Rest)
    }

    pub fn is_sustain(&self) -> bool {
        self.pitch_chain.len() == 1 && matches!(self.pitch_chain[0], Pitch::Sustain)
    }
}

#[derive(Debug, Clone, strum::EnumTryAs, strum::IntoStaticStr)]
pub enum EventBody {
    Note(Note),
    BaseNoteDef(PitchSpell),
    BaseFequencyDef(f32),
    TimeSignatureDef(Rational32),
    BeatDurationDef(Rational32),
    BPMDef(f32),
    QuantizeDef(Rational32),
    NewMeasure(u32),
}
#[derive(Debug, Clone)]
pub struct CompileEvent {
    pub body: EventBody,
    pub start_time: TimeStamp,
    pub range: TextRange,
    pub range_invoked: Option<TextRange>,
}

pub struct MacroRegistry {
    pub alias_macros: HashMap<String, Vec<Pitch>>,
    pub simple_macros: HashMap<String, Vec<Note>>,
    pub complex_macros: HashMap<String, Vec<CompileEvent>>,
}

pub struct CompileState {
    pub time: TimeStamp,
    pub base_note: PitchSpell,
    pub base_frequency: f32,
    pub time_signature: Rational32,
    pub beat_duration: Rational32,
    pub bpm: f32,
    pub quantize: Rational32,
    pub edo_def: u16,
}

impl CompileState {
    pub fn new() -> Self {
        Self {
            time: TimeStamp::default(),
            base_note: 60,          // C4
            base_frequency: 261.63, // Frequency of C4
            time_signature: Rational32::new(4, 4),
            beat_duration: Rational32::new(1, 4),
            bpm: 120.0,
            quantize: Rational32::new(1, 4),
            edo_def: 0,
        }
    }
}

impl Default for MacroRegistry {
    fn default() -> Self {
        Self {
            alias_macros: HashMap::new(),
            simple_macros: HashMap::new(),
            complex_macros: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DiagnosticLevel {
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub message: String,
    pub span: TextRange,
}
