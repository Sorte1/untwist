import { flagsFrom } from '../settings/flags';
import { parseHash } from '../settings/hash';
import { modeLabel, modeOf } from '../settings/mode';
import type { InputState } from '../state';

export const RECENT_LIMIT = 10;
export const RECENT_TEXT_CAP = 512 * 1024;
export const RECENT_TOTAL_CAP = 2_000_000;

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;
const WEEK = 7 * DAY;

export interface RecentEntry {
  readonly name: string;
  readonly text: string;
  readonly bytes: number;
  readonly time: number;
  readonly hash: string;
}

export function entryOf(input: InputState, hash: string, time: number): RecentEntry {
  return { name: input.name, text: input.text, bytes: input.bytes, time, hash };
}

function withinTotal(list: readonly RecentEntry[]): RecentEntry[] {
  const kept: RecentEntry[] = [];
  let total = 0;
  for (const item of list) {
    total += item.text.length;
    if (total > RECENT_TOTAL_CAP) {
      break;
    }
    kept.push(item);
  }
  return kept;
}

export function remembered(list: readonly RecentEntry[], next: RecentEntry): readonly RecentEntry[] {
  if (next.text === '' || next.text.length > RECENT_TEXT_CAP) {
    return list;
  }
  const others = list.filter((item) => item.text !== next.text);
  return withinTotal([next, ...others].slice(0, RECENT_LIMIT));
}

function isEntry(value: unknown): value is RecentEntry {
  if (typeof value !== 'object' || value === null) {
    return false;
  }
  const item = value as Record<string, unknown>;
  return (
    typeof item['name'] === 'string' &&
    typeof item['text'] === 'string' &&
    typeof item['bytes'] === 'number' &&
    typeof item['time'] === 'number' &&
    typeof item['hash'] === 'string'
  );
}

export function parseRecent(raw: string | null): RecentEntry[] {
  if (raw === null) {
    return [];
  }
  try {
    const value: unknown = JSON.parse(raw);
    return Array.isArray(value) ? value.filter(isEntry).map(({ name, text, bytes, time, hash }) => ({ name, text, bytes, time, hash })) : [];
  } catch {
    return [];
  }
}

export function ago(time: number, now: number): string {
  const span = now - time;
  if (span < MINUTE) {
    return 'just now';
  }
  if (span < HOUR) {
    return `${Math.floor(span / MINUTE)} min ago`;
  }
  if (span < DAY) {
    return `${Math.floor(span / HOUR)} h ago`;
  }
  if (span < 2 * DAY) {
    return 'yesterday';
  }
  if (span < WEEK) {
    return `${Math.floor(span / DAY)} days ago`;
  }
  return new Date(time).toLocaleDateString('en-US', { month: 'short', day: 'numeric', timeZone: 'UTC' });
}

export function settingsLine(hash: string): string {
  const settings = parseHash(hash).settings;
  const flags = flagsFrom(settings)
    .split(' ')
    .filter((word) => word !== '' && word !== '--unsafe')
    .join(' ');
  const mode = modeLabel(modeOf(settings));
  return flags === '' ? mode : `${mode} · ${flags}`;
}
