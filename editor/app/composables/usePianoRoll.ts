import type { EditorState } from "@codemirror/state";
import type { EditorView } from "@codemirror/view";
import * as d3 from "d3";
import { getEvents } from "~/utils/cm";
import type { NoteEvent } from "~/utils/cm";
import { getNoteColorFromFreq } from "~/utils/colors";
import { useSmoothWheelScroll } from "~/composables/useSmoothWheelScroll";
import { playNotesInSelection, subscribePlaybackState } from "~/utils/cm";

type RenderNote = {
    id: string;
    x: number;
    y: number;
    width: number;
    height: number;
    text: string;
    color: string;
};

type RenderMeasure = {
    id: string;
    x: number;
    bar: number;
};

type RenderBaseLine = {
    id: string;
    x1: number;
    x2: number;
    y: number;
    isPrimary: boolean;
};

function safeSemitoneOf(freq: number): number | null {
    if (!Number.isFinite(freq) || freq <= 0) return null;
    return 69 + 12 * Math.log2(freq / 440);
}

export function usePianoRoll({ editorState, editorView, containerRef, svgRef }: {
    editorState: Ref<EditorState | null>,
    editorView: Ref<EditorView | null>,
    containerRef: Ref<HTMLElement | null>,
    svgRef: Ref<SVGSVGElement | null>,
}) {
    const DEFAULT_BASE_FREQ = 261.63;
    const ZOOM_INTENSITY = 0.0025;
    const MIN_PIXEL_PER_SECOND = 10;
    const MAX_PIXEL_PER_SECOND = 600;
    const MIN_PIXEL_PER_SEMITONE = 4;
    const MAX_PIXEL_PER_SEMITONE = 60;
    const layoutParams = toReactive<{
        pixelPerSecond: number;
        pixelPerSemitone: number;
        anchorSecond: number;
        anchorSemitone: number;
    }>(useLocalStorage('piano-roll-layout', {
        pixelPerSecond: 50,
        pixelPerSemitone: 8,
        anchorSecond: 0,
        anchorSemitone: 0,
    }));
    const { height, width } = useElementSize(containerRef);
    const NEG_MARGIN_PIXEL = 200;
    const latestEvents = shallowRef<NoteEvent[]>([]);

    const contentSize = reactive({
        width: 1,
        height: 1,
    });

    const panState = reactive({
        active: false,
        startClientX: 0,
        startClientY: 0,
        startScrollLeft: 0,
        startScrollTop: 0,
    });
    const pointerInside = ref(false);
    let prevBodyUserSelect = "";

    const axisBounds = reactive({
        minSecond: 0,
        maxSecond: 1,
        minSemitone: 48,
        maxSemitone: 72,
    });

    const cursorVisual = reactive({
        visible: false,
        second: 0,
        x: 0,
    });
    const playbackCursorFallback = reactive({
        active: false,
        startSecond: 0,
        startMs: 0,
    });
    const playbackStatus = reactive({
        isPlaying: false,
    });
    let rafId: number | null = null;
    let lastSyncedEditorCursorPos: number | null = null;
    let lastCursorHead = -1;
    let didInitialBaseFreqCenter = false;

    useSmoothWheelScroll({
        scrollRef: containerRef,
        shouldHandleWheel: (event) => !event.ctrlKey && !event.altKey,
        preventDefaultWhenIgnored: true,
        wheelDirection: "horizontal",
    });

    function clamp(value: number, min: number, max: number): number {
        return Math.min(max, Math.max(min, value));
    }

    function shouldSuppressAltMenuFocus(): boolean {
        const element = containerRef.value;
        if (!element) return false;
        const active = document.activeElement;
        const focusedInPianoRoll = !!active && (active === element || element.contains(active));
        return pointerInside.value || focusedInPianoRoll;
    }

    function applyZoom({
        event,
        axis,
    }: {
        event: WheelEvent;
        axis: "x" | "y";
    }) {
        const element = containerRef.value;
        if (!element) return;

        event.preventDefault();

        const rect = element.getBoundingClientRect();
        const localX = event.clientX - rect.left;
        const localY = event.clientY - rect.top;

        if (axis === "x") {
            const prevScale = layoutParams.pixelPerSecond;
            const anchorSecond = axisBounds.minSecond + (element.scrollLeft + localX) / Math.max(prevScale, 1);
            const zoomFactor = Math.exp(-event.deltaY * ZOOM_INTENSITY);
            const nextScale = clamp(prevScale * zoomFactor, MIN_PIXEL_PER_SECOND, MAX_PIXEL_PER_SECOND);
            if (Math.abs(nextScale - prevScale) < 0.001) return;

            layoutParams.pixelPerSecond = nextScale;
            redraw();

            const targetLeft = (anchorSecond - axisBounds.minSecond) * nextScale - localX;
            const maxLeft = Math.max(element.scrollWidth - element.clientWidth, 0);
            element.scrollLeft = clamp(targetLeft, 0, maxLeft);
        } else {
            const prevScale = layoutParams.pixelPerSemitone;
            const anchorSemitone = axisBounds.maxSemitone - (element.scrollTop + localY) / Math.max(prevScale, 1);
            const zoomFactor = Math.exp(-event.deltaY * ZOOM_INTENSITY);
            const nextScale = clamp(prevScale * zoomFactor, MIN_PIXEL_PER_SEMITONE, MAX_PIXEL_PER_SEMITONE);
            if (Math.abs(nextScale - prevScale) < 0.001) return;

            layoutParams.pixelPerSemitone = nextScale;
            redraw();

            const targetTop = (axisBounds.maxSemitone - anchorSemitone) * nextScale - localY;
            const maxTop = Math.max(element.scrollHeight - element.clientHeight, 0);
            element.scrollTop = clamp(targetTop, 0, maxTop);
        }

        draw(latestEvents.value);
    }

    function updateAxisBounds(events: NoteEvent[]) {
        const noteHeight = Math.max(layoutParams.pixelPerSemitone, 1);

        if (events.length === 0) {
            const safeWidth = Math.max(width.value, 600);
            const safeHeight = Math.max(height.value, 240);
            const secSpan = safeWidth / layoutParams.pixelPerSecond;
            const semitoneSpan = safeHeight / layoutParams.pixelPerSemitone;

            axisBounds.minSecond = layoutParams.anchorSecond;
            axisBounds.maxSecond = layoutParams.anchorSecond + secSpan;
            axisBounds.minSemitone = layoutParams.anchorSemitone;
            axisBounds.maxSemitone = layoutParams.anchorSemitone + semitoneSpan;
        } else {
            let minSecond = Number.POSITIVE_INFINITY;
            let maxSecond = Number.NEGATIVE_INFINITY;
            let minSemitone = Number.POSITIVE_INFINITY;
            let maxSemitone = Number.NEGATIVE_INFINITY;

            for (const event of events) {
                const start = event.start_sec;
                const end = event.start_sec + event.duration_sec;
                minSecond = Math.min(minSecond, start);
                maxSecond = Math.max(maxSecond, end);

                if (event.type !== "Note") continue;
                const semitone = safeSemitoneOf(event.freq);
                if (semitone == null) continue;
                minSemitone = Math.min(minSemitone, semitone);
                maxSemitone = Math.max(maxSemitone, semitone);
            }

            if (!Number.isFinite(minSecond) || !Number.isFinite(maxSecond) || !Number.isFinite(minSemitone) || !Number.isFinite(maxSemitone)) {
                const safeWidth = Math.max(width.value, 600);
                const safeHeight = Math.max(height.value, 240);
                const secSpan = safeWidth / layoutParams.pixelPerSecond;
                const semitoneSpan = safeHeight / layoutParams.pixelPerSemitone;

                axisBounds.minSecond = layoutParams.anchorSecond;
                axisBounds.maxSecond = layoutParams.anchorSecond + secSpan;
                axisBounds.minSemitone = layoutParams.anchorSemitone;
                axisBounds.maxSemitone = layoutParams.anchorSemitone + semitoneSpan;
            } else {
                const marginSecond = NEG_MARGIN_PIXEL / layoutParams.pixelPerSecond;
                const marginSemitone = NEG_MARGIN_PIXEL / layoutParams.pixelPerSemitone;

                axisBounds.minSecond = minSecond - marginSecond;
                axisBounds.maxSecond = maxSecond + marginSecond;
                axisBounds.minSemitone = Math.floor(minSemitone - marginSemitone);
                axisBounds.maxSemitone = Math.ceil(maxSemitone + marginSemitone);
            }
        }

        const computedWidth = (axisBounds.maxSecond - axisBounds.minSecond) * layoutParams.pixelPerSecond;
        const computedHeight = (axisBounds.maxSemitone - axisBounds.minSemitone) * layoutParams.pixelPerSemitone + noteHeight;

        contentSize.width = Math.max(computedWidth, width.value || 1);
        contentSize.height = Math.max(computedHeight, height.value || 1);
    }

    function toRenderNotes(events: NoteEvent[], editorState: EditorState): RenderNote[] {
        const noteHeight = Math.max(layoutParams.pixelPerSemitone, 1);
        const baseDefs = events
            .filter((event) => event.type === "BaseFrequencyDef" && Number.isFinite(event.freq) && event.freq > 0)
            .sort((a, b) => a.start_sec - b.start_sec);

        let baseDefIndex = 0;
        let currentBaseFreq = DEFAULT_BASE_FREQ;

        return events.filter(e => e.type === 'Note').flatMap((event, index) => {
            const semitone = safeSemitoneOf(event.freq);
            if (semitone == null) return [];

            while (baseDefIndex < baseDefs.length && baseDefs[baseDefIndex]!.start_sec <= event.start_sec) {
                currentBaseFreq = baseDefs[baseDefIndex]!.freq;
                baseDefIndex += 1;
            }

            const x = (event.start_sec - axisBounds.minSecond) * layoutParams.pixelPerSecond;
            const y = (axisBounds.maxSemitone - semitone) * layoutParams.pixelPerSemitone;

            return [{
                id: `${event.start_sec}-${event.freq}-${event.span_from}-${event.span_to}-${index}`,
                x,
                y: Math.max(y, 0),
                width: Math.max(event.duration_sec * layoutParams.pixelPerSecond, 1),
                height: noteHeight,
                text: editorState.sliceDoc(event.span_from, event.span_to),
                color: getNoteColorFromFreq(event.freq, currentBaseFreq),
            }];
        });
    }

    function toRenderMeasures(events: NoteEvent[]): RenderMeasure[] {
        return [{
            type: "NewMeasure",
            start_bar: 0,
            start_sec: 0,
        }, ...events]
            .filter((event) => event.type === "NewMeasure")
            .map((event, index) => ({
                id: `${event.start_bar}-${event.start_sec}-${index}`,
                x: (event.start_sec - axisBounds.minSecond) * layoutParams.pixelPerSecond,
                bar: event.start_bar + 1,
            }));
    }

    function toRenderBaseLines(events: NoteEvent[]): RenderBaseLine[] {
        const baseDefs = events
            .filter((event) => event.type === "BaseFrequencyDef" && Number.isFinite(event.freq) && event.freq > 0)
            .sort((a, b) => a.start_sec - b.start_sec);

        const segments: Array<{ startSec: number; endSec: number; baseFreq: number }> = [];

        if (baseDefs.length === 0) {
            segments.push({
                startSec: axisBounds.minSecond,
                endSec: axisBounds.maxSecond,
                baseFreq: DEFAULT_BASE_FREQ,
            });
        } else {
            let cursorSec = axisBounds.minSecond;
            let currentBaseFreq = DEFAULT_BASE_FREQ;

            for (const def of baseDefs) {
                if (def.start_sec <= axisBounds.minSecond) {
                    currentBaseFreq = def.freq;
                    continue;
                }

                if (def.start_sec > cursorSec) {
                    segments.push({
                        startSec: cursorSec,
                        endSec: Math.min(def.start_sec, axisBounds.maxSecond),
                        baseFreq: currentBaseFreq,
                    });
                }

                currentBaseFreq = def.freq;
                cursorSec = Math.max(cursorSec, def.start_sec);
                if (cursorSec >= axisBounds.maxSecond) break;
            }

            if (cursorSec < axisBounds.maxSecond) {
                segments.push({
                    startSec: cursorSec,
                    endSec: axisBounds.maxSecond,
                    baseFreq: currentBaseFreq,
                });
            }
        }

        const result: RenderBaseLine[] = [];
        const semitoneMin = axisBounds.minSemitone;
        const semitoneMax = axisBounds.maxSemitone;

        for (let segmentIndex = 0; segmentIndex < segments.length; segmentIndex += 1) {
            const seg = segments[segmentIndex]!;
            const baseSemitone = safeSemitoneOf(seg.baseFreq);
            if (baseSemitone == null) continue;

            const octaveOffsetMin = Math.ceil((semitoneMin - baseSemitone) / 12);
            const octaveOffsetMax = Math.floor((semitoneMax - baseSemitone) / 12);

            for (let octaveOffset = octaveOffsetMin; octaveOffset <= octaveOffsetMax; octaveOffset += 1) {
                const semitone = baseSemitone + octaveOffset * 12;
                const y = (axisBounds.maxSemitone - semitone) * layoutParams.pixelPerSemitone;

                result.push({
                    id: `${segmentIndex}-${octaveOffset}-${seg.startSec}-${seg.endSec}`,
                    x1: (seg.startSec - axisBounds.minSecond) * layoutParams.pixelPerSecond,
                    x2: (seg.endSec - axisBounds.minSecond) * layoutParams.pixelPerSecond,
                    y,
                    isPrimary: octaveOffset === 0,
                });
            }
        }

        return result;
    }

    function draw(events: NoteEvent[]) {
        if (!svgRef.value || !containerRef.value || !toValue(editorState)) return;

        const viewportWidth = Math.max(width.value, 1);
        const viewportHeight = Math.max(height.value, 1);
        const scrollLeft = containerRef.value.scrollLeft;
        const scrollTop = containerRef.value.scrollTop;

        const virtualLeft = scrollLeft - NEG_MARGIN_PIXEL;
        const virtualRight = scrollLeft + viewportWidth + NEG_MARGIN_PIXEL;
        const virtualTop = scrollTop - NEG_MARGIN_PIXEL;
        const virtualBottom = scrollTop + viewportHeight + NEG_MARGIN_PIXEL;

        const renderNotes = toRenderNotes(events, toValue(editorState)!).filter((note) => {
            const right = note.x + note.width;
            const bottom = note.y + note.height;
            return !(right < virtualLeft || note.x > virtualRight || bottom < virtualTop || note.y > virtualBottom);
        });
        const renderMeasures = toRenderMeasures(events).filter((measure) => {
            return !(measure.x < virtualLeft || measure.x > virtualRight);
        });
        const renderBaseLines = toRenderBaseLines(events).filter((line) => {
            return !(line.x2 < virtualLeft || line.x1 > virtualRight || line.y < virtualTop || line.y > virtualBottom);
        });

        const svg = d3
            .select(svgRef.value)
            .attr("width", viewportWidth)
            .attr("height", viewportHeight)
            .attr("viewBox", `0 0 ${viewportWidth} ${viewportHeight}`);

        const measureLines = svg
            .selectAll<SVGLineElement, RenderMeasure>("line.measure-line")
            .data(renderMeasures, (d) => d.id);

        measureLines
            .join(
                (enter) => enter.append("line").attr("class", "measure-line"),
                (update) => update,
                (exit) => exit.remove(),
            )
            .attr("x1", (d) => d.x - scrollLeft)
            .attr("x2", (d) => d.x - scrollLeft)
            .attr("y1", -scrollTop)
            .attr("y2", contentSize.height - scrollTop)
            .attr("stroke", "currentColor")
            .attr("stroke-opacity", 0.22)
            .attr("stroke-width", 1);

        const measureLabels = svg
            .selectAll<SVGTextElement, RenderMeasure>("text.measure-label")
            .data(renderMeasures, (d) => d.id);

        measureLabels
            .join(
                (enter) => enter.append("text").attr("class", "measure-label"),
                (update) => update,
                (exit) => exit.remove(),
            )
            .attr("x", (d) => d.x - scrollLeft + 4)
            .attr("y", 14)
            .attr("font-size", 11)
            .attr("fill", "currentColor")
            .attr("opacity", 0.8)
            .attr("pointer-events", "none")
            .attr("text-anchor", "start")
            .attr("dominant-baseline", "middle")
            .text((d) => `${d.bar}`);


        const baseLines = svg
            .selectAll<SVGLineElement, RenderBaseLine>("line.base-frequency-line")
            .data(renderBaseLines, (d) => d.id);

        baseLines
            .join(
                (enter) => enter.append("line").attr("class", "base-frequency-line"),
                (update) => update,
                (exit) => exit.remove(),
            )
            .attr("x1", (d) => d.x1 - scrollLeft)
            .attr("x2", (d) => d.x2 - scrollLeft)
            .attr("y1", (d) => d.y - scrollTop)
            .attr("y2", (d) => d.y - scrollTop)
            .attr("stroke", "#94a3b8")
            .attr("stroke-opacity", (d) => (d.isPrimary ? 0.7 : 0.38))
            .attr("stroke-width", (d) => (d.isPrimary ? 1.4 : 0.8));

        const cursorGlow = svg
            .selectAll<SVGLineElement, number>("line.cursor-line-glow")
            .data(cursorVisual.visible ? [cursorVisual.x] : []);

        cursorGlow
            .join(
                (enter) => enter.append("line").attr("class", "cursor-line-glow"),
                (update) => update,
                (exit) => exit.remove(),
            )
            .attr("x1", (x) => x - scrollLeft)
            .attr("x2", (x) => x - scrollLeft)
            .attr("y1", -scrollTop)
            .attr("y2", contentSize.height - scrollTop)
            .attr("stroke", "#22c55e")
            .attr("stroke-opacity", 0.28)
            .attr("stroke-width", 8);

        const cursorLine = svg
            .selectAll<SVGLineElement, number>("line.cursor-line")
            .data(cursorVisual.visible ? [cursorVisual.x] : []);

        cursorLine
            .join(
                (enter) => enter.append("line").attr("class", "cursor-line"),
                (update) => update,
                (exit) => exit.remove(),
            )
            .attr("x1", (x) => x - scrollLeft)
            .attr("x2", (x) => x - scrollLeft)
            .attr("y1", -scrollTop)
            .attr("y2", contentSize.height - scrollTop)
            .attr("stroke", "#22c55e")
            .attr("stroke-opacity", 0.95)
            .attr("stroke-width", 1.6);

        const notes = svg
            .selectAll<SVGRectElement, RenderNote>("rect.note-rect")
            .data(renderNotes, (d) => d.id);

        notes
            .join(
                (enter) => enter.append("rect").attr("class", "note-rect"),
                (update) => update,
                (exit) => exit.remove(),
            )
            .attr("x", (d) => d.x - scrollLeft)
            .attr("y", (d) => d.y - scrollTop - d.height / 2)
            .attr("width", (d) => d.width)
            .attr("height", (d) => d.height)
            .attr("rx", 2)
            .attr("ry", 2)
            .attr("fill", (d) => d.color)
            .attr("opacity", 0.8)

        const noteLabels = svg
            .selectAll<SVGTextElement, RenderNote>("text.note-label")
            .data(renderNotes, (d) => d.id);
        noteLabels
            .join(
                (enter) => enter.append("text").attr("class", "note-label"),
                (update) => update,
                (exit) => exit.remove(),
            )
            .attr("x", (d) => d.x - scrollLeft + 4)
            .attr("y", (d) => d.y - scrollTop)
            .attr("font-size", (d) => Math.max(8, d.height * 0.72))
            .attr("fill", "white")
            .attr("pointer-events", "none")
            .attr("text-anchor", "start")
            .attr("dominant-baseline", "middle")
            .text((d) => d.text)
            .each(function (d) {
                const textEl = d3.select(this);
                const horizontalPadding = 4;
                const minLabelHeight = 10;
                const availableWidth = Math.max(d.width - horizontalPadding * 2, 0);
                const labelWidth = (this as SVGTextElement).getComputedTextLength();
                const show = d.height >= minLabelHeight && labelWidth <= availableWidth;
                textEl.attr("display", show ? null : "none");
            });
    }

    function getEventSpan(event: NoteEvent): { from: number; to: number } {
        const from = event.span_invoked_from ?? event.span_from;
        const to = event.span_invoked_to ?? event.span_to;
        return from <= to ? { from, to } : { from: to, to: from };
    }

    function findNearestEventByCursorPos(events: NoteEvent[], pos: number): NoteEvent | null {
        let nearest: NoteEvent | null = null;
        let bestDistance = Number.POSITIVE_INFINITY;
        let bestRange = Number.POSITIVE_INFINITY;

        for (const event of events) {
            const { from, to } = getEventSpan(event);
            const distance = pos < from ? from - pos : pos > to ? pos - to : 0;
            const range = to - from;

            if (distance < bestDistance || (distance === bestDistance && range < bestRange)) {
                bestDistance = distance;
                bestRange = range;
                nearest = event;
            }
        }

        return nearest;
    }

    function findNearestEventBySecond(events: NoteEvent[], second: number): NoteEvent | null {
        const noteEvents = events.filter((event) => event.type === "Note");
        let nearest: NoteEvent | null = null;
        let bestDistance = Number.POSITIVE_INFINITY;

        for (const event of noteEvents) {
            const distance = Math.abs(event.start_sec - second);
            if (distance < bestDistance) {
                bestDistance = distance;
                nearest = event;
            }
        }

        return nearest;
    }

    function resolveBaseFreqAtZero(events: NoteEvent[]): number {
        const baseDefs = events
            .filter((event) => event.type === "BaseFrequencyDef" && Number.isFinite(event.freq) && event.freq > 0)
            .sort((a, b) => a.start_sec - b.start_sec);

        let baseFreq = DEFAULT_BASE_FREQ;
        for (const baseDef of baseDefs) {
            if (baseDef.start_sec > 0) break;
            baseFreq = baseDef.freq;
        }

        return baseFreq;
    }

    function centerBaseFreqLine(baseFreq: number) {
        const element = containerRef.value;
        if (!element) return;

        const baseSemitone = safeSemitoneOf(baseFreq);
        if (baseSemitone == null) return;

        const baseY = (axisBounds.maxSemitone - baseSemitone) * layoutParams.pixelPerSemitone;
        const maxTop = Math.max(element.scrollHeight - element.clientHeight, 0);
        const centeredTop = clamp(baseY - element.clientHeight / 2, 0, maxTop);
        element.scrollTop = centeredTop;
    }

    function centerBaseFreqOnInit(events: NoteEvent[]) {
        if (didInitialBaseFreqCenter) return;

        centerBaseFreqLine(resolveBaseFreqAtZero(events));
        didInitialBaseFreqCenter = true;
    }

    function syncCursorFromEditorState(state: EditorState, events: NoteEvent[]) {
        if (events.length === 0) {
            cursorVisual.visible = false;
            playbackCursorFallback.active = false;
            lastSyncedEditorCursorPos = null;
            return;
        }

        const cursorPos = state.selection.main.head;
        if (playbackStatus.isPlaying && lastSyncedEditorCursorPos === cursorPos) {
            return;
        }

        const nearest = findNearestEventByCursorPos(events, cursorPos);
        if (!nearest) {
            cursorVisual.visible = false;
            playbackCursorFallback.active = false;
            return;
        }

        lastSyncedEditorCursorPos = cursorPos;
        cursorVisual.visible = true;
        cursorVisual.second = nearest.start_sec;
        cursorVisual.x = (cursorVisual.second - axisBounds.minSecond) * layoutParams.pixelPerSecond;
        playbackCursorFallback.startSecond = cursorVisual.second;
        playbackCursorFallback.startMs = performance.now();
        if (playbackStatus.isPlaying) {
            startPlaybackCursorFallback();
        }
    }

    function stepPlaybackCursorFallback() {
        if (!playbackCursorFallback.active) {
            rafId = null;
            return;
        }
        if (!cursorVisual.visible) {
            rafId = null;
            return;
        }

        const elapsedSec = Math.max(0, (performance.now() - playbackCursorFallback.startMs) / 1000);
        cursorVisual.second = playbackCursorFallback.startSecond + elapsedSec;
        cursorVisual.x = (cursorVisual.second - axisBounds.minSecond) * layoutParams.pixelPerSecond;
        draw(latestEvents.value);
        scrollToCursorLineIfNeeded();

        rafId = window.requestAnimationFrame(stepPlaybackCursorFallback);
    }

    function startPlaybackCursorFallback() {
        if (!cursorVisual.visible) return;
        playbackCursorFallback.active = true;
        playbackCursorFallback.startSecond = cursorVisual.second;
        playbackCursorFallback.startMs = performance.now();
        if (rafId == null) {
            rafId = window.requestAnimationFrame(stepPlaybackCursorFallback);
        }
    }

    function stopPlaybackCursorFallback() {
        playbackCursorFallback.active = false;
        if (rafId != null) {
            window.cancelAnimationFrame(rafId);
            rafId = null;
        }
    }

    function scrollToCursorLineIfNeeded() {
        if (!cursorVisual.visible) return;
        const element = containerRef.value;
        if (!element) return;

        const margin = Math.min(120, Math.max(40, element.clientWidth * 0.15));
        const leftBound = element.scrollLeft + margin;
        const rightBound = element.scrollLeft + element.clientWidth - margin;
        const maxLeft = Math.max(element.scrollWidth - element.clientWidth, 0);

        if (cursorVisual.x < leftBound) {
            element.scrollLeft = clamp(cursorVisual.x - margin, 0, maxLeft);
        } else if (cursorVisual.x > rightBound) {
            element.scrollLeft = clamp(cursorVisual.x - element.clientWidth + margin, 0, maxLeft);
        }
    }

    function moveEditorCursorFromPianoRoll(event: MouseEvent) {
        if (!event.ctrlKey || event.button !== 0) return;
        const element = containerRef.value;
        const view = toValue(editorView);
        if (!element || !view) return;

        event.preventDefault();

        const rect = element.getBoundingClientRect();
        const localX = event.clientX - rect.left;
        const xInContent = element.scrollLeft + localX;
        const targetSecond = axisBounds.minSecond + xInContent / layoutParams.pixelPerSecond;

        const nearest = findNearestEventBySecond(latestEvents.value, targetSecond);
        if (!nearest) return;

        const { from } = getEventSpan(nearest);
        view.dispatch({
            selection: { anchor: from },
            scrollIntoView: true,
        });
        view.focus();
    }

    function startPan(event: MouseEvent) {
        if (event.button !== 1) return;
        const element = containerRef.value;
        if (!element) return;

        event.preventDefault();

        panState.active = true;
        panState.startClientX = event.clientX;
        panState.startClientY = event.clientY;
        panState.startScrollLeft = element.scrollLeft;
        panState.startScrollTop = element.scrollTop;

        prevBodyUserSelect = document.body.style.userSelect;
        document.body.style.userSelect = "none";
        element.style.cursor = "grabbing";
    }

    function movePan(event: MouseEvent) {
        if (!panState.active) return;
        const element = containerRef.value;
        if (!element) return;

        event.preventDefault();

        const deltaX = event.clientX - panState.startClientX;
        const deltaY = event.clientY - panState.startClientY;

        element.scrollLeft = panState.startScrollLeft - deltaX;
        element.scrollTop = panState.startScrollTop - deltaY;
    }

    function endPan() {
        if (!panState.active) return;
        panState.active = false;

        const element = containerRef.value;
        if (element) {
            element.style.cursor = "";
        }
        document.body.style.userSelect = prevBodyUserSelect;
    }

    function redraw() {
        updateAxisBounds(latestEvents.value);
        draw(latestEvents.value);
    }

    useEventListener(containerRef, "scroll", () => {
        draw(latestEvents.value);
    });

    useEventListener(containerRef, "mouseenter", () => {
        pointerInside.value = true;
    });

    useEventListener(containerRef, "mouseleave", () => {
        pointerInside.value = false;
    });

    useEventListener(containerRef, "mousedown", startPan);
    useEventListener(containerRef, "mousedown", moveEditorCursorFromPianoRoll);
    useEventListener(containerRef, "auxclick", (event: MouseEvent) => {
        if (event.button === 1) {
            event.preventDefault();
        }
    });
    useEventListener(window, "mousemove", movePan);
    useEventListener(window, "mouseup", endPan);
    useEventListener(window, "blur", endPan);

    useEventListener(
        window,
        "keydown",
        (event: KeyboardEvent) => {
            if (event.key !== "Alt") return;
            if (!shouldSuppressAltMenuFocus()) return;
            event.preventDefault();
        },
        { capture: true },
    );

    useEventListener(
        window,
        "keyup",
        (event: KeyboardEvent) => {
            if (event.key !== "Alt") return;
            if (!shouldSuppressAltMenuFocus()) return;
            event.preventDefault();
        },
        { capture: true },
    );

    useEventListener(containerRef, "keydown", (event: KeyboardEvent) => {
        const isSpace = event.code === "Space" || event.key === " ";
        if (!isSpace || event.ctrlKey || event.altKey || event.metaKey || event.shiftKey) return;
        if (event.repeat) return;

        event.preventDefault();

        const view = toValue(editorView);
        if (!view) return;
        playNotesInSelection(view);
    });

    useEventListener(
        containerRef,
        "wheel",
        (event: WheelEvent) => {
            if (event.ctrlKey) {
                applyZoom({ event, axis: "x" });
                return;
            }
            if (event.altKey) {
                applyZoom({ event, axis: "y" });
            }
        },
        { passive: false },
    );

    watch([width, height], () => {
        redraw();
    });

    const unsubscribePlaybackState = subscribePlaybackState(({ isPlaying }) => {
        playbackStatus.isPlaying = isPlaying;
        if (isPlaying) {
            startPlaybackCursorFallback();
            return;
        }
        stopPlaybackCursorFallback();
    });

    watch(
        editorState,
        (newState) => {
            if (!newState) {
                latestEvents.value = [];
                cursorVisual.visible = false;
                stopPlaybackCursorFallback();
                lastSyncedEditorCursorPos = null;
                lastCursorHead = -1;
                didInitialBaseFreqCenter = false;
                redraw();
                return;
            }
            latestEvents.value = getEvents(newState);
            syncCursorFromEditorState(newState, latestEvents.value);
            redraw();

            if (!didInitialBaseFreqCenter) {
                nextTick(() => {
                    centerBaseFreqOnInit(latestEvents.value);
                });
            }

            const currentHead = newState.selection.main.head;
            if (lastCursorHead !== -1 && currentHead !== lastCursorHead) {
                scrollToCursorLineIfNeeded();
            }
            lastCursorHead = currentHead;
        },
        { immediate: true },
    );

    onBeforeUnmount(() => {
        stopPlaybackCursorFallback();
        unsubscribePlaybackState();
        endPan();
    });

    return {
        contentWidth: computed(() => contentSize.width),
        contentHeight: computed(() => contentSize.height),
    };
}