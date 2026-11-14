import type { RunResult } from '../../engine/types';
import { sizeLabel } from '../../dom';
import { moduleLabel, rangeLabel, type LibraryRow } from '../../libraries/libraries';
import { locateModules, type ModuleHit } from '../../libraries/locate';
import { tableRows, tableStateText, type TableRow } from '../../libraries/panel-model';
import type { TableIndex } from '../../libraries/index-format';
import { durationLabel } from '../../progress';
import './libraries.css';

export interface LibrariesView {
  readonly libraries: boolean;
  readonly rows: readonly LibraryRow[];
  readonly result: RunResult | null;
  readonly index: TableIndex | null;
  readonly reveal: (hit: ModuleHit) => void;
  readonly fileName: (index: number) => string;
  readonly turnOn: () => void;
}

function make<K extends keyof HTMLElementTagNameMap>(tag: K, className: string, text = ''): HTMLElementTagNameMap[K] {
  const element = document.createElement(tag);
  element.className = className;
  if (text !== '') {
    element.textContent = text;
  }
  return element;
}

function emptyState(view: LibrariesView): HTMLElement {
  const box = make('div', 'lib-empty');
  box.append(
    make('p', 'lib-note', 'Library names is off. Turn it on to detect known libraries such as core-js, regenerator-runtime, tslib and the Babel helpers, put a comment above each library module, fold them in the output, and fetch the extra name tables a run needs.'),
    make('p', 'lib-note muted', 'It is off by default because the scan and the table downloads add time to every run. The setting is under Settings > Libraries.'),
  );
  const button = make('button', 'btn btn-small', 'Turn on and run again');
  button.type = 'button';
  button.id = 'libraries-turn-on';
  button.addEventListener('click', view.turnOn);
  box.append(button);
  return box;
}

function hitLabel(hit: ModuleHit, view: LibrariesView): string {
  const line = `line ${hit.line.toLocaleString('en-US')}`;
  return hit.file === null ? line : `${view.fileName(hit.file)}:${hit.line}`;
}

function moduleList(hits: readonly ModuleHit[], view: LibrariesView): HTMLElement {
  if (hits.length === 0) {
    return make('p', 'lib-note muted', 'No module comment of this library is in the output text.');
  }
  const list = make('ol', 'lib-modules');
  for (const hit of hits) {
    const item = make('li', 'lib-module');
    const button = make('button', 'lib-jump');
    button.type = 'button';
    button.append(make('span', 'lib-module-name mono', hit.module), make('span', 'lib-module-at muted', hitLabel(hit, view)));
    button.title = `Show ${hit.module} in the output`;
    button.addEventListener('click', () => view.reveal(hit));
    item.append(button);
    list.append(item);
  }
  return list;
}

function libraryItem(row: LibraryRow, view: LibrariesView, hits: () => Map<string, ModuleHit[]>): HTMLElement {
  const item = make('details', 'lib-item');
  item.dataset['library'] = row.library;
  const summary = make('summary', 'lib-summary');
  summary.append(make('span', 'lib-name', row.library), make('span', 'lib-range mono', rangeLabel(row)), make('span', 'lib-count muted', moduleLabel(row.modules)));
  item.append(summary);
  item.addEventListener('toggle', () => {
    if (item.open && item.childElementCount === 1) {
      item.append(moduleList(hits().get(row.library) ?? [], view));
    }
  });
  return item;
}

function tableItem(row: TableRow): HTMLElement {
  const item = make('tr', 'lib-table-row');
  item.dataset['state'] = row.state;
  item.append(
    make('td', 'lib-table-name', row.label),
    make('td', 'lib-table-state', tableStateText(row)),
    make('td', 'lib-table-size mono', row.bytes === null ? '—' : sizeLabel(row.bytes)),
    make('td', 'lib-table-ms mono', row.ms === null ? '—' : durationLabel(row.ms)),
  );
  return item;
}

function tablesSection(view: LibrariesView): HTMLElement {
  const section = make('section', 'lib-tables');
  section.append(make('h3', 'lib-title', 'Name tables'));
  const rows = tableRows(view.index, view.result?.libraryTables);
  if (rows.length === 0) {
    section.append(make('p', 'lib-note muted', 'The table index could not be read.'));
    return section;
  }
  const table = make('table', 'lib-table');
  const head = make('tr', 'lib-table-head');
  head.append(make('th', '', 'Table'), make('th', '', 'State'), make('th', '', 'Size'), make('th', '', 'Load'));
  table.append(head, ...rows.map(tableItem));
  section.append(table);
  const used = view.result?.libraryTables;
  if (used !== undefined) {
    section.append(make('p', 'lib-note muted', `Input and output scan ${durationLabel(used.scanMs)}, loading ${durationLabel(used.loadMs)}${used.reran ? '; found in the output, so the run went again with the table' : ''}.`));
  }
  return section;
}

function cachedHits(result: RunResult | null, rows: readonly LibraryRow[]): () => Map<string, ModuleHit[]> {
  let found: Map<string, ModuleHit[]> | null = null;
  return () => {
    found ??= result === null ? new Map() : locateModules(result.stdout, result.modules, new Set(rows.map((row) => row.library)));
    return found;
  };
}

export function renderLibraries(holder: HTMLElement, view: LibrariesView): void {
  if (!view.libraries && view.rows.length === 0) {
    holder.replaceChildren(emptyState(view));
    return;
  }
  const list = make('section', 'lib-list');
  list.append(make('h3', 'lib-title', 'Detected libraries'));
  if (view.result === null) {
    list.append(make('p', 'lib-note muted', 'Run something to see the libraries in it.'));
  } else if (view.rows.length === 0) {
    list.append(make('p', 'lib-note muted', 'No known library code was found in this output.'));
  } else {
    const total = view.rows.reduce((sum, row) => sum + row.modules, 0);
    list.append(make('p', 'lib-note muted', `${view.rows.length} ${view.rows.length === 1 ? 'library' : 'libraries'}, ${moduleLabel(total)}. Open a library to list its modules; click one to show it in the output.`));
    const hits = cachedHits(view.result, view.rows);
    list.append(...view.rows.map((row) => libraryItem(row, view, hits)));
  }
  holder.replaceChildren(list, tablesSection(view));
}
