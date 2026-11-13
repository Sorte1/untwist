import { describe, expect, it } from 'vitest';
import { ago, entryOf, parseRecent, RECENT_LIMIT, RECENT_TEXT_CAP, RECENT_TOTAL_CAP, remembered, settingsLine, type RecentEntry } from './recent';

const MINUTE = 60_000;

function entry(text: string, time: number, name = 'a.js'): RecentEntry {
  return { name, text, bytes: text.length, time, hash: '' };
}

describe('remembered', () => {
  it('puts the newest run first', () => {
    const list = remembered([entry('a', 1)], entry('b', 2));
    expect(list.map((item) => item.text)).toEqual(['b', 'a']);
  });

  it('moves a rerun of the same text to the top with its latest name, time and settings', () => {
    const first = remembered([entry('b', 2), entry('a', 1)], { ...entry('a', 3, 'renamed.js'), hash: '#rename=unique' });
    expect(first).toEqual([{ ...entry('a', 3, 'renamed.js'), hash: '#rename=unique' }, entry('b', 2)]);
  });

  it(`keeps at most ${RECENT_LIMIT} runs`, () => {
    let list: readonly RecentEntry[] = [];
    for (let index = 0; index < RECENT_LIMIT + 3; index += 1) {
      list = remembered(list, entry(`t${index}`, index));
    }
    expect(list).toHaveLength(RECENT_LIMIT);
    expect(list[0]?.text).toBe(`t${RECENT_LIMIT + 2}`);
    expect(list.at(-1)?.text).toBe('t3');
  });

  it('drops the oldest runs once the kept text passes the total cap', () => {
    const big = 'x'.repeat(RECENT_TEXT_CAP);
    let list: readonly RecentEntry[] = [];
    for (let index = 0; index < 6; index += 1) {
      list = remembered(list, entry(`${index}${big.slice(1)}`, index));
    }
    expect(list.reduce((sum, item) => sum + item.text.length, 0)).toBeLessThanOrEqual(RECENT_TOTAL_CAP);
    expect(list[0]?.time).toBe(5);
    expect(list.length).toBe(Math.floor(RECENT_TOTAL_CAP / RECENT_TEXT_CAP));
  });

  it('skips an empty input and one over the per-run cap', () => {
    const list = [entry('a', 1)];
    expect(remembered(list, entry('', 2))).toBe(list);
    expect(remembered(list, entry('x'.repeat(RECENT_TEXT_CAP + 1), 2))).toBe(list);
  });
});

describe('parseRecent', () => {
  it('reads back what was written', () => {
    const list = [entry('a', 1), { ...entry('b', 2, 'b.js'), hash: '#unsafe=0' }];
    expect(parseRecent(JSON.stringify(list))).toEqual(list);
  });

  it('gives an empty list for nothing, broken JSON or a different shape', () => {
    expect(parseRecent(null)).toEqual([]);
    expect(parseRecent('{nope')).toEqual([]);
    expect(parseRecent('{"a":1}')).toEqual([]);
  });

  it('drops entries with missing or wrong fields', () => {
    const raw = JSON.stringify([entry('a', 1), { name: 'b.js', text: 3 }, null, { ...entry('c', 2), hash: 7 }]);
    expect(parseRecent(raw)).toEqual([entry('a', 1)]);
  });
});

describe('entryOf', () => {
  it('takes the input and the settings hash at the run', () => {
    const input = { text: 'var a;', name: 'x.js', bytes: 6, lines: 1, large: false, revision: 3 };
    expect(entryOf(input, '#rename=unique', 42)).toEqual({ name: 'x.js', text: 'var a;', bytes: 6, time: 42, hash: '#rename=unique' });
  });
});

describe('ago', () => {
  const now = Date.UTC(2026, 10, 3, 12, 0, 0);
  it('reads like a person would say it', () => {
    expect(ago(now - 10_000, now)).toBe('just now');
    expect(ago(now - 5 * MINUTE, now)).toBe('5 min ago');
    expect(ago(now - 3 * 60 * MINUTE, now)).toBe('3 h ago');
    expect(ago(now - 26 * 60 * MINUTE, now)).toBe('yesterday');
    expect(ago(now - 3 * 24 * 60 * MINUTE, now)).toBe('3 days ago');
    expect(ago(Date.UTC(2026, 8, 14, 12), now)).toBe('Sep 14');
    expect(ago(now + MINUTE, now)).toBe('just now');
  });
});

describe('settingsLine', () => {
  it('names the mode and the flags that differ from it', () => {
    expect(settingsLine('#unsafe=0')).toBe('Safe');
    expect(settingsLine('')).toBe('Unsafe');
    expect(settingsLine('#unsafe=0&rename=unique&theme=light')).toBe('Safe · --rename unique');
    expect(settingsLine('#unpack=0&fuel=1000')).toBe('Unsafe · --no-unpack --fuel 1000');
    expect(settingsLine('#libs=1&rename=smart')).toBe('Unsafe · library names · --rename smart');
  });
});
