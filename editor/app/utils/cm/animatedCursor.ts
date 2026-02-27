import { type Extension } from "@codemirror/state";
import { EditorView, ViewPlugin, ViewUpdate } from "@codemirror/view";
import gsap from "gsap";

const CURSOR_WIDTH = 2;
const ANIM_DURATION = 0.1;

export function createAnimatedCursorTheme(): Extension {
    return EditorView.baseTheme({
        ".cm-cursor": {
            display: "none",
        },
        ".cm-cursorLayer": {
            display: "none",
        },
        ".cm-scroller": {
            position: "relative",
        },
        ".cm-symi-ghost-cursor": {
            position: "absolute",
            width: `${CURSOR_WIDTH}px`,
            backgroundColor: "#7CFF7A",
            borderRadius: "1px",
            pointerEvents: "none",
            zIndex: "10",
        },
        ".cm-symi-ghost-cursor-primary": {
            boxShadow: "0 0 1px 1px lime",
            opacity: "1",
        },
        ".cm-symi-ghost-cursor-secondary": {
            boxShadow: "0 0 1px 1px darklime",
            opacity: "0.7",
        },
    });
}

export function createAnimatedCursor(): Extension {
    return ViewPlugin.fromClass(class {
        private cursorEls: HTMLDivElement[] = [];
        private lastPosByIndex = new Map<number, number>();
        private pendingMeasure = false;

        constructor(private view: EditorView) {
            this.scheduleMeasure(view);
        }

        update(update: ViewUpdate) {
            if (update.selectionSet || update.viewportChanged || update.focusChanged) {
                this.scheduleMeasure(update.view);
            }
        }

        destroy() {
            for (const cursorEl of this.cursorEls) {
                cursorEl.remove();
            }
            this.cursorEls = [];
            this.lastPosByIndex.clear();
        }

        private ensureCursorCount(count: number) {
            while (this.cursorEls.length < count) {
                const cursorEl = document.createElement("div");
                cursorEl.className = "cm-symi-ghost-cursor";
                this.view.scrollDOM.appendChild(cursorEl);
                this.cursorEls.push(cursorEl);
            }
            while (this.cursorEls.length > count) {
                const cursorEl = this.cursorEls.pop();
                cursorEl?.remove();
            }
            for (let index = count; index <= this.lastPosByIndex.size; index++) {
                this.lastPosByIndex.delete(index);
            }
        }

        private scheduleMeasure(view: EditorView) {
            if (this.pendingMeasure) return;
            this.pendingMeasure = true;
            view.requestMeasure({
                read: (view) => {
                    if (!view.hasFocus) return { visible: false, cursors: [] as Array<{ index: number; pos: number; x: number; y: number; height: number; isPrimary: boolean }> };
                    const scrollRect = view.scrollDOM.getBoundingClientRect();
                    const cursors: Array<{ index: number; pos: number; x: number; y: number; height: number; isPrimary: boolean }> = [];

                    view.state.selection.ranges.forEach((range, index) => {
                        const pos = range.head;
                        const coords = view.coordsAtPos(pos, 1);
                        if (!coords) return;
                        cursors.push({
                            index,
                            pos,
                            x: coords.left - scrollRect.left + view.scrollDOM.scrollLeft,
                            y: coords.top - scrollRect.top + view.scrollDOM.scrollTop,
                            height: Math.max(1, coords.bottom - coords.top),
                            isPrimary: index === view.state.selection.mainIndex,
                        });
                    });

                    if (cursors.length === 0) return { visible: false, cursors };
                    return {
                        visible: true,
                        cursors,
                    };
                },
                write: (measure) => {
                    this.pendingMeasure = false;
                    if (!measure || !measure.visible) {
                        this.ensureCursorCount(0);
                        return;
                    }

                    const sorted = [...measure.cursors].sort((a, b) => a.index - b.index);
                    this.ensureCursorCount(sorted.length);

                    sorted.forEach((cursor, index) => {
                        const cursorEl = this.cursorEls[index]!;
                        cursorEl.className = `cm-symi-ghost-cursor ${cursor.isPrimary ? "cm-symi-ghost-cursor-primary" : "cm-symi-ghost-cursor-secondary"}`;
                        cursorEl.style.height = `${cursor.height}px`;

                        const lastPos = this.lastPosByIndex.get(index);
                        if (lastPos === cursor.pos && cursorEl.style.opacity !== "0") {
                            return;
                        }
                        this.lastPosByIndex.set(index, cursor.pos);

                        gsap.to(cursorEl, {
                            duration: ANIM_DURATION,
                            x: cursor.x,
                            y: cursor.y,
                            overwrite: true,
                            ease: "power2.out",
                        });
                    });
                },
            });
        }
    });
}
