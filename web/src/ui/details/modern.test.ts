import { describe, expect, it } from 'vitest';
import { groupTitle, modernGroups, modernSummary } from './modern';

describe('the modern section of a report', () => {
  it('reads the groups and drops malformed ones', () => {
    const report = JSON.stringify({
      modern: [
        { status: 'kept', reason: 'it calls code that is not recognised as core-js', items: ['core-js 3.39: modules/es.string.fixed.js', 3] },
        { status: 'removed', reason: 'native in the baseline', items: ['core-js 3.39: modules/es.array.flat.js'] },
        { status: 'unknown', reason: 'x', items: [] },
        { status: 'kept', items: [] },
      ],
    });
    expect(modernGroups(report)).toEqual([
      { status: 'kept', reason: 'it calls code that is not recognised as core-js', items: ['core-js 3.39: modules/es.string.fixed.js'] },
      { status: 'removed', reason: 'native in the baseline', items: ['core-js 3.39: modules/es.array.flat.js'] },
    ]);
    expect(modernGroups('{}')).toEqual([]);
    expect(modernGroups('not json')).toEqual([]);
  });

  it('titles a group by status, count and reason', () => {
    expect(groupTitle({ status: 'kept', reason: 'it is not run as a statement', items: ['a', 'b'] })).toBe('kept 2: it is not run as a statement');
  });

  it('finds the one summary line among the modern notes', () => {
    const notes = [
      { level: 'info', pass: 'modern', kind: 'progress', message: 'the output assumes an engine at least as new as chrome 120' },
      { level: 'info', pass: 'modern', kind: 'progress', message: 'removed 58 of 129 core-js polyfill modules and 18 helper modules, kept 71 (41 need code not recognised as core-js)' },
    ] as const;
    expect(modernSummary(notes)).toMatch(/^removed 58 of 129/);
    expect(modernSummary([])).toBeNull();
  });
});
