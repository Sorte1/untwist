import './strings.css';
import { virtualList } from '../details/virtual-list';
import { decodedStrings, matching, quoted, type DecodedRow } from './model';

const ROW_HEIGHT = 27;
const VISIBLE_ROWS = 10;
const SEARCH_DELAY_MS = 120;

export interface StringsView {
  readonly json: string;
  readonly text: string;
  readonly linked: boolean;
  readonly reveal: (span: readonly [number, number]) => void;
}

function make<K extends keyof HTMLElementTagNameMap>(tag: K, className: string, text = ''): HTMLElementTagNameMap[K] {
  const element = document.createElement(tag);
  element.className = className;
  element.textContent = text;
  return element;
}

function count(value: number, word: string): string {
  return `${value.toLocaleString('en-US')} ${word}${value === 1 ? '' : 's'}`;
}

function drawRow(row: DecodedRow, view: StringsView): HTMLElement {
  const { span } = row;
  const linked = view.linked && span !== undefined;
  const element = make(linked ? 'button' : 'div', 'string-row');
  const value = quoted(row.value);
  element.append(make('span', 'string-loc', row.place), make('span', 'string-call', row.call), make('span', 'string-value', value));
  element.title = row.call === '' ? value : `${row.call} → ${value}`;
  if (element instanceof HTMLButtonElement && span !== undefined) {
    element.type = 'button';
    element.addEventListener('click', () => view.reveal(span));
  }
  return element;
}

export function renderStrings(holder: HTMLElement, view: StringsView): void {
  const table = decodedStrings(view.json, view.text);
  const search = make('input', 'string-search');
  search.type = 'search';
  search.id = 'strings-search';
  search.placeholder = 'Search values and calls';
  search.setAttribute('aria-label', 'Search the decoded strings');
  search.autocomplete = 'off';
  search.spellcheck = false;
  const summary = make('span', 'string-summary');
  summary.setAttribute('role', 'status');
  const head = make('div', 'strings-head');
  head.append(search, summary);
  const list = virtualList<DecodedRow>(ROW_HEIGHT, VISIBLE_ROWS, (row) => drawRow(row, view));
  list.element.classList.add('string-rows');
  const empty = make('p', 'string-note', 'No decoded string matches.');
  const notes = [
    ...(table.more > 0 ? [make('p', 'string-note', `${count(table.more, 'more site')} not listed.`)] : []),
    ...(view.linked ? [] : [make('p', 'string-note', 'The input changed after this run, so the places are not linked.')]),
  ];
  const show = () => {
    const rows = matching(table.rows, search.value);
    const total = `${count(table.rows.length, 'decoded string')} · ${table.distinct.toLocaleString('en-US')} distinct`;
    summary.textContent = rows.length === table.rows.length ? total : `${rows.length.toLocaleString('en-US')} of ${total}`;
    empty.hidden = rows.length > 0;
    list.setRows(rows);
  };
  let timer: ReturnType<typeof setTimeout> | undefined;
  search.addEventListener('input', () => {
    clearTimeout(timer);
    timer = setTimeout(show, table.rows.length > 5_000 ? SEARCH_DELAY_MS : 0);
  });
  holder.replaceChildren(head, list.element, empty, ...notes);
  show();
}
