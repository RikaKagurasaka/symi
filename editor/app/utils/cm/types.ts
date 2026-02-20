export type Diagnostic = {
    message: string;
    severity: "Warning" | "Error" | string;
    from: number;
    to: number;
};

export type NoteEvent = {
    type: "Note" | "NewMeasure" | "BaseFrequencyDef"
    freq: number;
    start_sec: number;
    start_bar: number;
    start_tick: [number, number];
    duration_sec: number;
    duration_tick: [number, number];
    span_from: number;
    span_to: number;
    span_invoked_from?: number;
    span_invoked_to?: number;
    pitch_ratio?: number;
};

export type ActiveNoteHighlight = {
    id: string;
    from: number;
    to: number;
    holdMs: number;
};
