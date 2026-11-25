import { describe, expect, it } from 'vitest';
import { restoredOf } from './components';

describe('restoredOf', () => {
  it('reads the restored files and the kept reasons', () => {
    const json = JSON.stringify({ files: [{ path: 'src/App.svelte', code: '<p>x</p>\n' }], kept: ['Nav kept as JS: a legacy (non-runes) component'] });
    expect(restoredOf(json)).toEqual({ files: [{ path: 'src/App.svelte', code: '<p>x</p>\n' }], kept: ['Nav kept as JS: a legacy (non-runes) component'] });
  });

  it('is empty for a run without the restore or with a broken reply', () => {
    expect(restoredOf(undefined)).toEqual({ files: [], kept: [] });
    expect(restoredOf('')).toEqual({ files: [], kept: [] });
    expect(restoredOf('{')).toEqual({ files: [], kept: [] });
    expect(restoredOf(JSON.stringify({ files: [{ path: 1 }], kept: [2] }))).toEqual({ files: [], kept: [] });
  });
});
