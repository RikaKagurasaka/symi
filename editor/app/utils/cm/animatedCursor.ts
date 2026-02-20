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
    });
}

export function createAnimatedCursor(): Extension {
    return ViewPlugin.fromClass(class {
        private cursorEl: HTMLDivElement;
        private lastPos: number | null = null;
        private pendingMeasure = false;

        constructor(private view: EditorView) {
            this.cursorEl = document.createElement("div");
            this.cursorEl.className = "cm-symi-ghost-cursor";
            this.view.scrollDOM.appendChild(this.cursorEl);
            this.scheduleMeasure(view);
        }

        update(update: ViewUpdate) {
            if (update.selectionSet || update.viewportChanged || update.focusChanged) {
                this.scheduleMeasure(update.view);
            }
        }

        destroy() {
            this.cursorEl.remove();
        }

        private scheduleMeasure(view: EditorView) {
            if (this.pendingMeasure) return;
            this.pendingMeasure = true;
            view.requestMeasure({
                read: (view) => {
                    if (!view.hasFocus) return { visible: false };
                    const pos = view.state.selection.main.head;
                    const coords = view.coordsAtPos(pos, 1);
                    if (!coords) return { visible: false };
                    const scrollRect = view.scrollDOM.getBoundingClientRect();
                    return {
                        visible: true,
                        pos,
                        x: coords.left - scrollRect.left + view.scrollDOM.scrollLeft,
                        y: coords.top - scrollRect.top + view.scrollDOM.scrollTop,
                        height: Math.max(1, coords.bottom - coords.top),
                    };
                },
                write: (measure) => {
                    this.pendingMeasure = false;
                    if (!measure || !measure.visible) {
                        this.cursorEl.style.opacity = "0";
                        return;
                    }
                    const pos = measure.pos ?? this.lastPos ?? 0;
                    if (pos === this.lastPos && this.cursorEl.style.opacity !== "0") return;
                    this.lastPos = pos;

                    this.cursorEl.style.height = `${measure.height}px`;
                    this.cursorEl.style.opacity = "1";

                    gsap.to(this.cursorEl, {
                        duration: ANIM_DURATION,
                        x: measure.x,
                        y: measure.y,
                        overwrite: true,
                        ease: "power2.out",
                    });
                },
            });
        }
    });
}
