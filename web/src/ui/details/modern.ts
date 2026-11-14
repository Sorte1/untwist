import { reportField } from '../../report-json';
import type { NoteRow } from './report';

export type ModernStatus = 'kept' | 'removed' | 'rewrote';

export interface ModernGroup {
  readonly status: ModernStatus;
  readonly reason: string;
  readonly items: readonly string[];
}

const STATUSES: readonly ModernStatus[] = ['kept', 'removed', 'rewrote'];
const SUMMARY = /^removed \d+ of \d+ core-js /;

function groupOf(value: unknown): ModernGroup | null {
  if (typeof value !== 'object' || value === null) {
    return null;
  }
  const { status, reason, items } = value as Record<string, unknown>;
  if (!STATUSES.includes(status as ModernStatus) || typeof reason !== 'string' || !Array.isArray(items)) {
    return null;
  }
  return { status: status as ModernStatus, reason, items: items.filter((item): item is string => typeof item === 'string') };
}

export function modernGroups(report: string): ModernGroup[] {
  const field = reportField(report, 'modern');
  return Array.isArray(field) ? field.map(groupOf).filter((group): group is ModernGroup => group !== null) : [];
}

export function modernSummary(notes: readonly NoteRow[]): string | null {
  return notes.find((note) => note.pass === 'modern' && SUMMARY.test(note.message))?.message ?? null;
}

export function groupTitle(group: ModernGroup): string {
  return `${group.status} ${group.items.length.toLocaleString('en-US')}: ${group.reason}`;
}

function make(tag: string, className: string, text = ''): HTMLElement {
  const element = document.createElement(tag);
  element.className = className;
  element.textContent = text;
  return element;
}

export function modernBody(summary: string | null, groups: readonly ModernGroup[]): HTMLElement {
  const body = make('div', 'modern-body');
  if (summary !== null) {
    body.append(make('p', 'modern-summary', summary));
  }
  for (const group of groups) {
    const item = make('details', 'modern-group');
    item.dataset['status'] = group.status;
    item.append(make('summary', 'modern-title', groupTitle(group)));
    const list = make('ul', 'modern-items mono');
    list.append(...group.items.map((text) => make('li', '', text)));
    item.append(list);
    body.append(item);
  }
  return body;
}
