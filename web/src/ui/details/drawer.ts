import './details.css';
import type { RunResult } from '../../engine/types';
import { debugBundle, downloadText } from './export';
import { parseReport, stageTotalMs, type NoteRow, type Report, type StageRow } from './report';
import { diagnosticsOf } from '../diagnostics-model';
import { librariesOf } from '../../libraries/libraries';
import { detectionPairs, libraryPairs, limitPairs, tablePairs, ms, passesOf, renamePairs, resourcePairs, warningClassPairs, type Pair } from './rows';
import { virtualList } from './virtual-list';

const ROW_HEIGHT = 26;
const VISIBLE_ROWS = 12;

type StageLine =
  | { readonly kind: 'stage'; readonly index: number; readonly stage: StageRow; readonly open: boolean }
  | { readonly kind: 'pass'; readonly name: string; readonly changes: number; readonly time: number };

function make(tag: string, className: string, text = ''): HTMLElement {
  const element = document.createElement(tag);
  element.className = className;
  element.textContent = text;
  return element;
}

function section(title: string, ...body: HTMLElement[]): HTMLElement {
  const element = make('section', 'details-section');
  element.append(make('h3', 'label', title), ...body);
  return element;
}

function pairs(rows: readonly Pair[]): HTMLElement {
  const list = make('dl', 'details-pairs');
  for (const row of rows) {
    list.append(make('dt', '', row.label), make('dd', '', row.value));
  }
  return list;
}

function stageLines(stages: readonly StageRow[], open: ReadonlySet<number>): StageLine[] {
  return stages.flatMap((stage, index): StageLine[] => {
    const head: StageLine = { kind: 'stage', index, stage, open: open.has(index) };
    if (!open.has(index)) {
      return [head];
    }
    return [head, ...passesOf(stage).map(([name, changes, time]): StageLine => ({ kind: 'pass', name, changes, time }))];
  });
}

function stageTable(stages: readonly StageRow[]): HTMLElement {
  const open = new Set<number>();
  const cells = (values: string[], className: string) => {
    const row = make('div', className);
    row.append(...values.map((value) => make('span', '', value)));
    return row;
  };
  const list = virtualList<StageLine>(ROW_HEIGHT, VISIBLE_ROWS, (line) => {
    if (line.kind === 'pass') {
      return cells(['', `· ${line.name}`, '', String(line.changes), '', ms(line.time)], 'stage-row pass-row');
    }
    const { stage } = line;
    const row = cells(
      [line.open ? '▾' : '▸', stage.stage, String(stage.round), String(stage.iterations), String(stage.changes), stage.stop, ms(stage.elapsed_ms)],
      'stage-row',
    );
    row.setAttribute('role', 'button');
    row.tabIndex = 0;
    row.setAttribute('aria-expanded', String(line.open));
    row.addEventListener('click', () => {
      open.has(line.index) ? open.delete(line.index) : open.add(line.index);
      list.setRows(stageLines(stages, open));
    });
    return row;
  });
  list.setRows(stageLines(stages, open));
  const header = cells(['', 'stage', 'round', 'iterations', 'changes', 'stop', 'elapsed'], 'stage-row stage-head');
  const wrap = make('div', 'stage-table');
  wrap.append(header, list.element);
  return wrap;
}

function refusals(notes: readonly NoteRow[]): HTMLElement {
  const refused = notes.filter((note) => note.kind === 'refusal');
  if (refused.length === 0) {
    return make('p', 'diag-note', 'No refusals.');
  }
  const list = virtualList<NoteRow>(ROW_HEIGHT, VISIBLE_ROWS, (note) => {
    const row = make('div', 'note-row');
    row.append(make('span', 'mono muted', note.pass), make('span', '', note.message));
    return row;
  });
  list.setRows(refused);
  return list.element;
}

function exportBar(result: RunResult, report: Report, flags: string): HTMLElement {
  const bar = make('div', 'details-actions');
  const download = make('button', 'btn btn-small', 'Download report.json') as HTMLButtonElement;
  download.type = 'button';
  download.id = 'download-report';
  download.addEventListener('click', () => downloadText('report.json', result.report, 'application/json'));
  const copy = make('button', 'btn btn-small', 'Copy debug bundle') as HTMLButtonElement;
  copy.type = 'button';
  copy.id = 'copy-debug';
  copy.addEventListener('click', () => {
    navigator.clipboard.writeText(debugBundle(flags, result.report, report.engine)).then(
      () => (copy.textContent = 'Copied'),
      () => (copy.textContent = 'Copy failed'),
    );
  });
  bar.append(download, copy);
  return bar;
}

export function renderDrawer(container: HTMLElement, result: RunResult | null, flags: string): void {
  const report = result === null ? null : parseReport(result.report);
  if (result === null || report === null) {
    container.replaceChildren(make('p', 'diag-note', 'Run something to see its report.'));
    return;
  }
  const stageMs = stageTotalMs(report);
  container.replaceChildren(
    exportBar(result, report, flags),
    section('Stages', stageTable(report.stages)),
    make('div', 'details-grid'),
  );
  container.lastElementChild?.append(
    section('Detection', pairs(detectionPairs(report))),
    section('Libraries', pairs([...libraryPairs(librariesOf(result.report)), ...tablePairs(result.libraryTables)])),
    section('Limits', pairs(limitPairs(report))),
    section('Rename', pairs(renamePairs(report))),
    section('Warnings by class', pairs(warningClassPairs(diagnosticsOf(result.report, result.stderr)))),
    section('Resources', pairs(resourcePairs(report, result, stageMs))),
    section('Refusals', refusals(report.notes)),
  );
}
