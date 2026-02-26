/*
* symi转midi接收以下参数：
*   - 输入events: Vec<CompileEvent>
*   - 弯音事件最大半音数定义(RPN): u16
*   - MIDI事件分辨率: u32
*   - 时间容差（秒）: f64
*   - 音高容差（音分）: f64
*
* 具体操作流程为：
*  1. 先收集所有拍号、BPM变化事件，构建“基于秒反推MIDI tick”的时间转换关系，并构建元事件列表
*    - 拍号使用 TimeSignatureDef；若分母不是2的幂，立即返回错误并中断导出
*    - BPM由 BeatDurationDef + BPMDef 共同定义：
*      BeatDurationDef 定义“以什么音符为一拍”，BPMDef 定义“一分钟有多少拍”
*      需要转换成MIDI支持的“每分钟四分音符拍数（quarter-note BPM）”后写入Tempo元事件
*  2. 将所有NoteEvent在时间轴上布局，具体规则如下：
*    - 原则上每个Track在同一时刻只能有一个激活的NoteEvent
*    - 允许例外：满足“可同轨合并”条件时，同一Track同一时刻可以有多个NoteEvent
*    - 任何NoteEvent优先放入index更低的Track
*    - 在“优先低index轨道”基础上，若存在并列可选方案，再选择使该Track与上一个NoteEvent音高跳变最小的方案
*    - 如果一个NoteEven
*
t在某个Track上与已有的NoteEvent时间重叠，则将其放入下一个Track，直到找到可放置Track
*    - 如果两个NoteEvent重叠的时长小于时间容差(秒)，则将更早的NoteEvent的结束时间调整为更晚的NoteEvent的开始时间，以消除重叠，并放在同一Track上
*    - 对于每个NoteEvent，根据频率计算MIDI note number和Pitch Bend值
*    - 若两个或多个同时开始的NoteEvent，其Pitch Bend对应音分差小于音高容差，则可同轨合并，Pitch Bend取平均值
*    - Rest事件直接忽略，不生成NoteOn/NoteOff
*    - 全局使用同一个RPN Pitch Bend Range设置
*  3. 将所有元事件和NoteEvent转换为MIDI事件，按时间顺序排序，输出SMF Format 1标准MIDI文件Buffer
*/
use anyhow::{Result, bail};
use midly::{
    Format, Header, MetaMessage, MidiMessage, PitchBend, Smf, Timing, TrackEvent, TrackEventKind,
    num::{u4, u7, u14, u15, u24, u28},
};

use crate::compiler::{
    rational::Rational32,
    types::{CompileEvent, EventBody, Note},
};

#[derive(Debug, Clone, Copy)]
pub struct MidiWriterConfig {
    pub pitch_bend_range_semitones: u16,
    pub ticks_per_quarter: u32,
    pub time_tolerance_seconds: f64,
    pub pitch_tolerance_cents: f64,
}

impl Default for MidiWriterConfig {
    fn default() -> Self {
        Self {
            pitch_bend_range_semitones: 2,
            ticks_per_quarter: 480,
            time_tolerance_seconds: 1e-4,
            pitch_tolerance_cents: 3.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct TempoPoint {
    second: f64,
    mpq: u32,
    start_tick: u64,
}

#[derive(Debug, Clone, Copy)]
struct RawTempoPoint {
    second: f64,
    mpq: u32,
}

#[derive(Debug, Clone, Copy)]
struct MetaPoint {
    second: f64,
    numerator: u8,
    denominator: u8,
}

#[derive(Debug, Clone, Copy)]
struct NoteSpec {
    start_second: f64,
    end_second: f64,
    midi_key: u8,
    bend14: u16,
    bend_cents: f64,
}

#[derive(Debug, Clone)]
struct NoteGroup {
    start_second: f64,
    end_second: f64,
    bend14: u16,
    bend_cents: f64,
    notes: Vec<NoteSpec>,
}

#[derive(Debug, Clone)]
struct TrackLayout {
    groups: Vec<NoteGroup>,
}

#[derive(Debug, Clone)]
struct AbsEvent {
    tick: u64,
    priority: u8,
    kind: TrackEventKind<'static>,
}

const PITCH_BEND_CENTER: i32 = 8192;
const PITCH_BEND_MIN_SIGNED: i32 = -8192;
const PITCH_BEND_MAX_SIGNED: i32 = 8191;

pub fn export_smf_format1(events: &[CompileEvent], config: MidiWriterConfig) -> Result<Vec<u8>> {
    let tpq = normalize_tpq(config.ticks_per_quarter)?;
    let (raw_tempos, time_signatures) = collect_tempo_and_signature(events)?;
    let tempo_points = build_tempo_points(&raw_tempos, tpq);

    let mut note_specs = collect_note_specs(events, config.pitch_bend_range_semitones)?;
    note_specs.sort_by(|a, b| {
        a.start_second
            .total_cmp(&b.start_second)
            .then_with(|| a.midi_key.cmp(&b.midi_key))
    });

    let grouped = build_same_start_groups(note_specs, config.pitch_tolerance_cents);
    let layouts = assign_groups_to_tracks(grouped, config.time_tolerance_seconds);

    if layouts.len() > 16 {
        bail!("Too many note tracks ({}) for MIDI channels", layouts.len());
    }

    let mut tracks: Vec<Vec<TrackEvent<'static>>> = Vec::new();
    tracks.push(build_meta_track(&tempo_points, &time_signatures, tpq));
    for (channel, layout) in layouts.iter().enumerate() {
        tracks.push(build_note_track(
            layout,
            channel as u8,
            config.pitch_bend_range_semitones,
            &tempo_points,
            tpq,
        ));
    }

    let smf = Smf {
        header: Header {
            format: Format::Parallel,
            timing: Timing::Metrical(u15::new(tpq)),
        },
        tracks,
    };

    let mut buffer = Vec::new();
    smf.write_std(&mut buffer)?;
    Ok(buffer)
}

fn normalize_tpq(tpq: u32) -> Result<u16> {
    if tpq == 0 {
        bail!("ticks_per_quarter must be > 0");
    }
    if tpq > 0x7FFF {
        bail!("ticks_per_quarter exceeds MIDI metrical range (32767)");
    }
    Ok(tpq as u16)
}

fn collect_tempo_and_signature(
    events: &[CompileEvent],
) -> Result<(Vec<RawTempoPoint>, Vec<MetaPoint>)> {
    let mut sorted = events.to_vec();
    sorted.sort_by(|a, b| a.start_time.seconds.total_cmp(&b.start_time.seconds));

    let mut beat_duration = Rational32::new(1, 4);
    let mut bpm = 120.0_f64;

    let mut raw_tempos: Vec<(f64, u32)> = vec![(0.0, bpm_beat_to_mpq(bpm, beat_duration)?)];
    let mut time_sigs = Vec::new();

    for event in &sorted {
        match event.body {
            EventBody::BeatDurationDef(dur) => {
                beat_duration = dur;
                raw_tempos.push((
                    event.start_time.seconds,
                    bpm_beat_to_mpq(bpm, beat_duration)?,
                ));
            }
            EventBody::BPMDef(next_bpm) => {
                bpm = next_bpm as f64;
                raw_tempos.push((
                    event.start_time.seconds,
                    bpm_beat_to_mpq(bpm, beat_duration)?,
                ));
            }
            EventBody::TimeSignatureDef(ts) => {
                let numerator = *ts.numer();
                let denominator = *ts.denom();
                if numerator <= 0 || denominator <= 0 {
                    bail!("Invalid time signature: {}/{}", numerator, denominator);
                }
                if !(denominator as u32).is_power_of_two() {
                    bail!(
                        "Time signature denominator {} is not a power of 2",
                        denominator
                    );
                }
                if numerator > u8::MAX as i32 || denominator > u8::MAX as i32 {
                    bail!(
                        "Time signature out of MIDI range: {}/{}",
                        numerator,
                        denominator
                    );
                }
                time_sigs.push(MetaPoint {
                    second: event.start_time.seconds,
                    numerator: numerator as u8,
                    denominator: denominator as u8,
                });
            }
            _ => {}
        }
    }

    raw_tempos.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut dedup: Vec<(f64, u32)> = Vec::new();
    for (sec, mpq) in raw_tempos {
        if let Some((last_sec, last_mpq)) = dedup.last_mut() {
            if (*last_sec - sec).abs() < 1e-9 {
                *last_mpq = mpq;
                continue;
            }
        }
        dedup.push((sec, mpq));
    }

    if dedup.first().is_none_or(|(sec, _)| *sec > 0.0) {
        dedup.insert(0, (0.0, bpm_beat_to_mpq(120.0, Rational32::new(1, 4))?));
    }

    let mut tempo_points = Vec::with_capacity(dedup.len());
    for (sec, mpq) in dedup {
        tempo_points.push(RawTempoPoint { second: sec, mpq });
    }

    Ok((tempo_points, time_sigs))
}

fn build_tempo_points(raw_points: &[RawTempoPoint], tpq: u16) -> Vec<TempoPoint> {
    let mut out = Vec::with_capacity(raw_points.len());
    let mut accum_tick = 0_u64;
    for (idx, point) in raw_points.iter().enumerate() {
        if idx > 0 {
            let prev = raw_points[idx - 1];
            let dt = (point.second - prev.second).max(0.0);
            accum_tick = accum_tick.saturating_add(seconds_to_ticks_with_mpq(dt, prev.mpq, tpq));
        }
        out.push(TempoPoint {
            second: point.second,
            mpq: point.mpq,
            start_tick: accum_tick,
        });
    }
    out
}

fn bpm_beat_to_mpq(bpm: f64, beat_duration: Rational32) -> Result<u32> {
    if bpm <= 0.0 {
        bail!("BPM must be > 0");
    }
    let beat_full_note = rational_to_f64(beat_duration)?;
    if beat_full_note <= 0.0 {
        bail!("BeatDurationDef must be > 0");
    }
    let quarter_bpm = bpm * (beat_full_note / 0.25);
    if quarter_bpm <= 0.0 {
        bail!("Derived quarter BPM must be > 0");
    }
    let mpq_f = 60_000_000.0 / quarter_bpm;
    let mpq = mpq_f.round().clamp(1.0, 16_777_215.0) as u32;
    Ok(mpq)
}

fn rational_to_f64(v: Rational32) -> Result<f64> {
    let d = *v.denom();
    if d == 0 {
        bail!("Rational denominator cannot be zero");
    }
    Ok(*v.numer() as f64 / d as f64)
}

fn collect_note_specs(events: &[CompileEvent], bend_range: u16) -> Result<Vec<NoteSpec>> {
    let mut notes = Vec::new();
    for event in events {
        let EventBody::Note(note) = &event.body else {
            continue;
        };
        if note.is_rest() {
            continue;
        }
        let spec = note_to_spec(event.start_time.seconds, note, bend_range)?;
        if spec.end_second > spec.start_second {
            notes.push(spec);
        }
    }
    Ok(notes)
}

fn note_to_spec(start_second: f64, note: &Note, bend_range: u16) -> Result<NoteSpec> {
    if note.freq <= 0.0 {
        bail!("Note frequency must be > 0 for MIDI export");
    }
    if note.duration_seconds <= 0.0 {
        bail!("Note duration_seconds must be > 0 for MIDI export");
    }
    let (midi_key, bend14, bend_cents) = freq_to_key_and_bend(note.freq as f64, bend_range)?;
    Ok(NoteSpec {
        start_second,
        end_second: start_second + note.duration_seconds,
        midi_key,
        bend14,
        bend_cents,
    })
}

fn freq_to_key_and_bend(freq: f64, bend_range: u16) -> Result<(u8, u16, f64)> {
    if bend_range == 0 {
        bail!("pitch_bend_range_semitones must be > 0");
    }
    let exact = 69.0 + 12.0 * (freq / 440.0).log2();
    let key = exact.round().clamp(0.0, 127.0) as u8;
    let semitone_delta = exact - f64::from(key);
    let bend_cents = semitone_delta * 100.0;
    let ratio = semitone_delta / bend_range as f64;
    let bend_signed = (ratio * PITCH_BEND_CENTER as f64).round() as i32;
    let bend14 = signed_to_bend14(bend_signed);
    Ok((key, bend14, bend_cents))
}

fn bend14_to_signed(bend14: u16) -> i32 {
    i32::from(bend14).clamp(0, 16383) - PITCH_BEND_CENTER
}

fn signed_to_bend14(bend_signed: i32) -> u16 {
    (bend_signed
        .clamp(PITCH_BEND_MIN_SIGNED, PITCH_BEND_MAX_SIGNED)
        + PITCH_BEND_CENTER) as u16
}

fn build_same_start_groups(mut notes: Vec<NoteSpec>, pitch_tolerance_cents: f64) -> Vec<NoteGroup> {
    notes.sort_by(|a, b| {
        a.start_second
            .total_cmp(&b.start_second)
            .then_with(|| a.bend_cents.total_cmp(&b.bend_cents))
    });

    let mut groups: Vec<NoteGroup> = Vec::new();
    for note in notes {
        if let Some(group) = groups.iter_mut().find(|group| {
            (group.start_second - note.start_second).abs() < 1e-9
                && (group.bend_cents - note.bend_cents).abs() <= pitch_tolerance_cents
        }) {
            group.notes.push(note);
            group.end_second = group.end_second.max(note.end_second);
            let n = group.notes.len() as f64;
            group.bend_cents = ((group.bend_cents * (n - 1.0)) + note.bend_cents) / n;
            let avg_signed = ((bend14_to_signed(group.bend14) as f64 * (n - 1.0))
                + bend14_to_signed(note.bend14) as f64)
                / n;
            group.bend14 = signed_to_bend14(avg_signed.round() as i32);
            continue;
        }

        groups.push(NoteGroup {
            start_second: note.start_second,
            end_second: note.end_second,
            bend14: note.bend14,
            bend_cents: note.bend_cents,
            notes: vec![note],
        });
    }

    groups.sort_by(|a, b| {
        a.start_second
            .total_cmp(&b.start_second)
            .then_with(|| a.notes.len().cmp(&b.notes.len()))
    });
    groups
}

fn assign_groups_to_tracks(groups: Vec<NoteGroup>, tolerance_seconds: f64) -> Vec<TrackLayout> {
    let mut tracks: Vec<TrackLayout> = Vec::new();

    for group in groups {
        let mut placed = false;
        for track in &mut tracks {
            let can_place = match track.groups.last_mut() {
                None => true,
                Some(last) => {
                    if group.start_second >= last.end_second {
                        true
                    } else {
                        let overlap = last.end_second - group.start_second;
                        if overlap > 0.0 && overlap <= tolerance_seconds {
                            last.end_second = group.start_second;
                            for n in &mut last.notes {
                                if n.end_second > group.start_second {
                                    n.end_second = group.start_second;
                                }
                            }
                            true
                        } else {
                            false
                        }
                    }
                }
            };

            if can_place {
                track.groups.push(group.clone());
                placed = true;
                break;
            }
        }

        if !placed {
            tracks.push(TrackLayout {
                groups: vec![group],
            });
        }
    }

    tracks
}

fn build_meta_track(
    tempo_points: &[TempoPoint],
    time_signatures: &[MetaPoint],
    tpq: u16,
) -> Vec<TrackEvent<'static>> {
    let mut abs_events = Vec::new();

    for tempo in tempo_points {
        let tick = seconds_to_tick(tempo.second, tempo_points, tpq);
        abs_events.push(AbsEvent {
            tick,
            priority: 0,
            kind: TrackEventKind::Meta(MetaMessage::Tempo(u24::new(tempo.mpq))),
        });
    }

    for sig in time_signatures {
        let tick = seconds_to_tick(sig.second, tempo_points, tpq);
        abs_events.push(AbsEvent {
            tick,
            priority: 1,
            kind: TrackEventKind::Meta(MetaMessage::TimeSignature(
                sig.numerator,
                sig.denominator.trailing_zeros() as u8,
                24,
                8,
            )),
        });
    }

    to_delta_track(abs_events)
}

fn build_note_track(
    layout: &TrackLayout,
    channel: u8,
    bend_range: u16,
    tempo_points: &[TempoPoint],
    tpq: u16,
) -> Vec<TrackEvent<'static>> {
    let mut abs_events = Vec::new();

    append_rpn_pitch_bend_setup(&mut abs_events, channel, bend_range);

    for group in &layout.groups {
        let start_tick = seconds_to_tick(group.start_second, tempo_points, tpq);
        abs_events.push(AbsEvent {
            tick: start_tick,
            priority: 1,
            kind: TrackEventKind::Midi {
                channel: u4::new(channel),
                message: MidiMessage::PitchBend {
                    bend: PitchBend(u14::new(group.bend14)),
                },
            },
        });

        for note in &group.notes {
            abs_events.push(AbsEvent {
                tick: start_tick,
                priority: 2,
                kind: TrackEventKind::Midi {
                    channel: u4::new(channel),
                    message: MidiMessage::NoteOn {
                        key: u7::new(note.midi_key),
                        vel: u7::new(100),
                    },
                },
            });

            let end_tick = seconds_to_tick(note.end_second, tempo_points, tpq).max(start_tick + 1);
            abs_events.push(AbsEvent {
                tick: end_tick,
                priority: 0,
                kind: TrackEventKind::Midi {
                    channel: u4::new(channel),
                    message: MidiMessage::NoteOff {
                        key: u7::new(note.midi_key),
                        vel: u7::new(0),
                    },
                },
            });
        }
    }

    to_delta_track(abs_events)
}

fn append_rpn_pitch_bend_setup(abs_events: &mut Vec<AbsEvent>, channel: u8, bend_range: u16) {
    let coarse = bend_range.min(127) as u8;
    let set_cc = |controller: u8, value: u8| AbsEvent {
        tick: 0,
        priority: 0,
        kind: TrackEventKind::Midi {
            channel: u4::new(channel),
            message: MidiMessage::Controller {
                controller: u7::new(controller),
                value: u7::new(value),
            },
        },
    };

    abs_events.push(set_cc(101, 0));
    abs_events.push(set_cc(100, 0));
    abs_events.push(set_cc(6, coarse));
    abs_events.push(set_cc(38, 0));
}

fn seconds_to_tick(second: f64, tempo_points: &[TempoPoint], tpq: u16) -> u64 {
    if tempo_points.is_empty() {
        return 0;
    }
    let mut idx = 0;
    for (i, tp) in tempo_points.iter().enumerate() {
        if tp.second <= second {
            idx = i;
        } else {
            break;
        }
    }
    let base = tempo_points[idx];
    let dt = (second - base.second).max(0.0);
    base.start_tick
        .saturating_add(seconds_to_ticks_with_mpq(dt, base.mpq, tpq))
}

fn seconds_to_ticks_with_mpq(second: f64, mpq: u32, tpq: u16) -> u64 {
    let ticks = second * (1_000_000.0 / mpq as f64) * tpq as f64;
    if ticks.is_finite() && ticks > 0.0 {
        ticks.round() as u64
    } else {
        0
    }
}

fn to_delta_track(mut abs_events: Vec<AbsEvent>) -> Vec<TrackEvent<'static>> {
    abs_events.sort_by(|a, b| {
        a.tick
            .cmp(&b.tick)
            .then_with(|| a.priority.cmp(&b.priority))
    });
    let mut out = Vec::with_capacity(abs_events.len() + 1);
    let mut cursor = 0_u64;
    for event in abs_events {
        let delta = event.tick.saturating_sub(cursor).min(0x0FFF_FFFF);
        out.push(TrackEvent {
            delta: u28::new(delta as u32),
            kind: event.kind,
        });
        cursor = event.tick;
    }
    out.push(TrackEvent {
        delta: u28::new(0),
        kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
    });
    out
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::{compiler::compile::Compiler, rowan::parse_fn::parse_source};

    #[test]
    fn export_compiled_events_to_smf1() {
        let source = Arc::from("(4/4)\n(120)\nC4:E4,\n");
        let parsed = parse_source(source);
        let mut compiler = Compiler::new();
        compiler.compile(&parsed.syntax_node());
        assert!(
            compiler
                .diagnostics
                .iter()
                .all(|d| { !matches!(d.level, crate::compiler::types::DiagnosticLevel::Error) }),
            "compiler has diagnostics: {:?}",
            compiler
                .diagnostics
                .iter()
                .map(|d| d.message.clone())
                .collect::<Vec<_>>()
        );

        let bytes = export_smf_format1(&compiler.events, MidiWriterConfig::default())
            .expect("midi export should succeed");
        assert!(!bytes.is_empty());

        let parsed_midi = Smf::parse(&bytes).expect("generated bytes should be valid SMF");
        assert_eq!(parsed_midi.header.format, Format::Parallel);
        assert!(parsed_midi.tracks.len() >= 2);

        let has_time_signature = parsed_midi.tracks[0].iter().any(|event| {
            matches!(
                event.kind,
                TrackEventKind::Meta(MetaMessage::TimeSignature(_, _, _, _))
            )
        });
        let has_tempo = parsed_midi.tracks[0]
            .iter()
            .any(|event| matches!(event.kind, TrackEventKind::Meta(MetaMessage::Tempo(_))));
        assert!(
            has_time_signature,
            "meta track should contain time signature"
        );
        assert!(has_tempo, "meta track should contain tempo");

        let has_note_on = parsed_midi.tracks.iter().skip(1).any(|track| {
            track.iter().any(|event| {
                matches!(
                    event.kind,
                    TrackEventKind::Midi {
                        message: MidiMessage::NoteOn { .. },
                        ..
                    }
                )
            })
        });
        assert!(has_note_on, "at least one note track should contain NoteOn");
    }

    #[test]
    fn test_same_start_note_groups() {
        let source = Arc::from("(4/4)\n(120)\n1/1:3/2,\n");
        let parsed = parse_source(source);
        let mut compiler = Compiler::new();
        compiler.compile(&parsed.syntax_node());
        let bytes = export_smf_format1(&compiler.events, MidiWriterConfig::default())
            .expect("midi export should succeed");
        let parsed_midi = Smf::parse(&bytes).expect("generated bytes should be valid SMF");
        let bends = parsed_midi
            .tracks
            .iter()
            .skip(1)
            .flat_map(|track| track.iter())
            .filter_map(|event| {
                if let TrackEventKind::Midi {
                    message: MidiMessage::PitchBend { bend },
                    ..
                } = event.kind
                {
                    Some(bend.0)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        println!("Extracted pitch bends: {:?}", bends);
    }

    #[test]
    fn pitch_bend_neutral_is_8192() {
        let (key, bend14, cents) = freq_to_key_and_bend(440.0, 2).expect("A4 should convert");
        assert_eq!(key, 69);
        assert_eq!(bend14, 8192);
        assert!(cents.abs() < 1e-9);
    }

    #[test]
    fn same_start_group_averages_bend_around_center() {
        let groups = build_same_start_groups(
            vec![
                NoteSpec {
                    start_second: 0.0,
                    end_second: 1.0,
                    midi_key: 60,
                    bend14: 8191,
                    bend_cents: -0.1,
                },
                NoteSpec {
                    start_second: 0.0,
                    end_second: 1.0,
                    midi_key: 64,
                    bend14: 8193,
                    bend_cents: 0.1,
                },
            ],
            1.0,
        );

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].bend14, 8192);
    }
}
