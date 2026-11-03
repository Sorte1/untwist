import { RangeSetBuilder, type EditorState, type Extension, type Text } from '@codemirror/state';
import { Decoration, EditorView, ViewPlugin, type DecorationSet, type ViewUpdate } from '@codemirror/view';

export const MAX_LEVEL = 24;
export const UNITS = [2, 4, 8] as const;
const SCAN_LINES = 200;
const SAMPLE_LINES = 400;
const CLOSERS = new Set(['}', ']', ')']);

export type Unit = (typeof UNITS)[number];

export function leadingColumns(text: string, tabSize: number): number | null {
  let columns = 0;
  for (const char of text) {
    if (char === ' ') {
      columns += 1;
    } else if (char === '\t') {
      columns += tabSize - (columns % tabSize);
    } else {
      return columns;
    }
  }
  return null;
}

export function detectUnit(doc: Text, tabSize: number): Unit {
  const last = Math.min(doc.lines, SAMPLE_LINES);
  let smallest = Number.POSITIVE_INFINITY;
  let previous = 0;
  for (let number = 1; number <= last; number += 1) {
    const columns = leadingColumns(doc.line(number).text, tabSize);
    if (columns === null) {
      continue;
    }
    const step = columns - previous;
    if (step > 0) {
      smallest = Math.min(smallest, step);
    }
    previous = columns;
  }
  return UNITS.find((unit) => smallest <= unit) ?? 8;
}

interface Measured {
  readonly level: number;
  readonly closes: boolean;
}

function measure(doc: Text, number: number, unit: number, tabSize: number): Measured | null {
  const text = doc.line(number).text;
  const columns = leadingColumns(text, tabSize);
  if (columns === null) {
    return null;
  }
  return { level: Math.min(MAX_LEVEL, Math.floor(columns / unit)), closes: CLOSERS.has(text.trimStart().charAt(0)) };
}

function nearest(doc: Text, from: number, step: 1 | -1, unit: number, tabSize: number): Measured | null {
  for (let number = from, scanned = 0; number >= 1 && number <= doc.lines && scanned < SCAN_LINES; number += step, scanned += 1) {
    const measured = measure(doc, number, unit, tabSize);
    if (measured !== null) {
      return measured;
    }
  }
  return null;
}

function blankLevel(above: Measured | null, below: Measured | null): number {
  if (above === null || below === null) {
    return 0;
  }
  return Math.min(above.level, below.closes ? below.level + 1 : below.level);
}

export function indentLevels(doc: Text, first: number, last: number, unit: number, tabSize: number): number[] {
  const levels: number[] = [];
  let above = nearest(doc, first - 1, -1, unit, tabSize);
  let blanks = 0;
  const settle = (below: Measured | null) => {
    const level = blankLevel(above, below);
    for (; blanks > 0; blanks -= 1) {
      levels.push(level);
    }
  };
  for (let number = first; number <= last; number += 1) {
    const measured = measure(doc, number, unit, tabSize);
    if (measured === null) {
      blanks += 1;
      continue;
    }
    settle(measured);
    levels.push(measured.level);
    above = measured;
  }
  settle(blanks > 0 ? nearest(doc, last + 1, 1, unit, tabSize) : null);
  return levels;
}

const lineMarks = Array.from({ length: MAX_LEVEL + 1 }, (_, level) => Decoration.line({ class: `cm-indent-g${level}` }));

function guides(view: EditorView, unit: number): DecorationSet {
  const { state } = view;
  const builder = new RangeSetBuilder<Decoration>();
  for (const { from, to } of view.visibleRanges) {
    const first = state.doc.lineAt(from).number;
    const last = state.doc.lineAt(to).number;
    const levels = indentLevels(state.doc, first, last, unit, state.tabSize);
    levels.forEach((level, offset) => {
      const mark = lineMarks[level];
      if (level > 0 && mark !== undefined) {
        const line = state.doc.line(first + offset);
        builder.add(line.from, line.from, mark);
      }
    });
  }
  return builder.finish();
}

function unitOf(state: EditorState): Unit {
  return detectUnit(state.doc, state.tabSize);
}

const guidePlugin = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;
    unit: Unit;

    constructor(view: EditorView) {
      this.unit = unitOf(view.state);
      this.decorations = guides(view, this.unit);
    }

    update(update: ViewUpdate): void {
      if (update.docChanged) {
        this.unit = unitOf(update.state);
      }
      if (update.docChanged || update.viewportChanged) {
        this.decorations = guides(update.view, this.unit);
      }
    }
  },
  { decorations: (plugin) => plugin.decorations },
);

function unitClass(view: EditorView): string {
  return `cm-indent-u${view.plugin(guidePlugin)?.unit ?? 2}`;
}

function guideTheme(): Extension {
  const spec: Record<string, Record<string, string>> = {
    '&': { '--guide': 'var(--border)' },
    '.cm-line': { backgroundRepeat: 'no-repeat', backgroundOrigin: 'content-box', backgroundPosition: '0 0' },
  };
  for (const unit of UNITS) {
    const image = `repeating-linear-gradient(to right, var(--guide) 0 1px, transparent 1px ${unit}ch)`;
    for (let level = 1; level <= MAX_LEVEL; level += 1) {
      spec[`&.cm-indent-u${unit} .cm-indent-g${level}`] = { backgroundImage: image, backgroundSize: `${level * unit}ch 100%` };
    }
  }
  return EditorView.theme(spec);
}

export function indentGuides(): Extension {
  return [guidePlugin, EditorView.editorAttributes.of((view) => ({ class: unitClass(view) })), guideTheme()];
}
