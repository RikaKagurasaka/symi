import { type Extension } from "@codemirror/state";
import { EditorView } from "@codemirror/view";

function getSelectionLineNumbers(view: EditorView): number[] {
	const lineNumbers = new Set<number>();
	const { doc, selection } = view.state;

	for (const range of selection.ranges) {
		const from = Math.min(range.anchor, range.head);
		const to = Math.max(range.anchor, range.head);
		const startLine = doc.lineAt(from).number;
		const endPos = to > from ? to - 1 : to;
		const endLine = doc.lineAt(endPos).number;

		for (let lineNo = startLine; lineNo <= endLine; lineNo++) {
			lineNumbers.add(lineNo);
		}
	}

	if (lineNumbers.size === 0) {
		lineNumbers.add(doc.lineAt(selection.main.head).number);
	}

	return [...lineNumbers].sort((a, b) => a - b);
}

const COMMENT_RE = /^(\s*)\/\/\s?/;
const INDENT_RE = /^(\s*)/;

function toggleLineComments(view: EditorView): boolean {
	const { doc } = view.state;
	const lineNumbers = getSelectionLineNumbers(view);
	if (lineNumbers.length === 0) return false;

	const allCommented = lineNumbers.every((lineNo) => {
		const line = doc.line(lineNo).text;
		return COMMENT_RE.test(line);
	});

	const changes = lineNumbers.map((lineNo) => {
		const line = doc.line(lineNo);
		if (allCommented) {
			const nextText = line.text.replace(COMMENT_RE, "$1");
			return { from: line.from, to: line.to, insert: nextText };
		}

		const indent = line.text.match(INDENT_RE)?.[1] ?? "";
		const nextText = `${indent}//${line.text.slice(indent.length)}`;
		return { from: line.from, to: line.to, insert: nextText };
	});

	view.dispatch({ changes });
	return true;
}

export function createCtrlSlashCommentHandler(): Extension {
	return EditorView.domEventHandlers({
		keydown(event, view) {
			const keyboardEvent = event as KeyboardEvent;
			const hasSingleMod = keyboardEvent.ctrlKey !== keyboardEvent.metaKey;
			const isModSlash = hasSingleMod
				&& !keyboardEvent.altKey
				&& (keyboardEvent.code === "Slash" || keyboardEvent.key === "/");
			if (!isModSlash) return false;
			event.preventDefault();
			return toggleLineComments(view);
		},
	});
}