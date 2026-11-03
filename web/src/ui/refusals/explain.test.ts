import { describe, expect, it } from 'vitest';
import { explainRefusals, sentenceFor, SHOWN_SITES } from './explain';

const SOURCE = "var k = 'x' + 1;\nconsole.log(b(0x0), window[k]);\nString.prototype.slice = function () {};";
const at = (needle: string): [number, number] => {
  const start = SOURCE.indexOf(needle);
  return [start, start + needle.length];
};

const report = (sites: unknown, notes: unknown[] = []) => JSON.stringify({ notes, refusalSites: sites });

describe('the refusal explanations', () => {
  it('say where each site is, quote its code and say why it kept the decoder', () => {
    const rows = explainRefusals(
      report([
        { what: 'BUILTIN_PATCHED: String.prototype.slice', layer: 0, inputSpan: at('String.prototype.slice') },
        { what: 'GLOBAL_REACH', layer: 0, inputSpan: at('window[k]') },
      ]),
      SOURCE,
    );
    expect(rows.shown).toEqual([
      {
        place: 'Line 2',
        line: 2,
        code: 'window[k]',
        sentence: "may reach the string array through the global object by a key that could be its name, so the decoder calls were left alone.",
        span: at('window[k]'),
      },
      {
        place: 'Line 3',
        line: 3,
        code: 'String.prototype.slice',
        sentence: "replaces String.prototype.slice, so the string decoder can't be trusted.",
        span: at('String.prototype.slice'),
      },
    ]);
    expect(rows.more).toBe(0);
  });

  it('keep one row per place and reason, and sites outside the input last and without a jump', () => {
    const rows = explainRefusals(
      report([
        { what: 'code from text', layer: 1 },
        { what: 'escaped read (definer)', layer: 0, inputSpan: at('window[k]') },
        { what: 'escaped read (definer runner)', layer: 0, inputSpan: at('window[k]') },
        { what: 'write by an unknown key', layer: 0 },
      ]),
      SOURCE,
    );
    expect(rows.shown.map((row) => [row.place, row.code, row.span])).toEqual([
      ['Line 2', 'window[k]', at('window[k]')],
      ['Unpacked code', '', undefined],
      ['Rewritten code', '', undefined],
    ]);
    expect(rows.shown[1]?.sentence).toMatch(/^runs code built from text/);
  });

  it('quote one line of code at most, cut to a readable length', () => {
    const long = `${'x'.repeat(200)}\nnext`;
    const rows = explainRefusals(report([{ what: 'GLOBAL_REACH', layer: 0, inputSpan: [0, long.length] }]), long);
    expect(rows.shown[0]?.code).toBe(`${'x'.repeat(80)}…`);
  });

  it('fall back to the refusal notes when no site was recorded', () => {
    const notes = [
      { level: 'warning', pass: 'inline-decoded-strings', kind: 'refusal', message: 'the program replaces a standard built-in', inputSpan: at('String.prototype.slice') },
      { level: 'info', pass: 'find-decoders', kind: 'progress', message: 'string array decoders found: 1' },
    ];
    const rows = explainRefusals(report(undefined, notes), SOURCE);
    expect(rows.shown).toEqual([
      { place: 'Line 3', line: 3, code: 'String.prototype.slice', sentence: 'the program replaces a standard built-in', span: at('String.prototype.slice') },
    ]);
  });

  it('show at most a fixed number of rows and count the rest', () => {
    const sites = Array.from({ length: SHOWN_SITES + 3 }, (_, index) => ({ what: `escaped read (definer)`, layer: 0, inputSpan: [index, index + 1] }));
    const rows = explainRefusals(report(sites), 'x'.repeat(SHOWN_SITES + 10));
    expect(rows.shown).toHaveLength(SHOWN_SITES);
    expect(rows.more).toBe(3);
  });

  it('read nothing from a broken report', () => {
    expect(explainRefusals('not json', SOURCE)).toEqual({ shown: [], more: 0 });
  });

  it('name every reason the engine records in plain words', () => {
    expect(sentenceFor('BUILTIN_PATCHED write: Array.prototype.push, from the patch proof')).toBe(
      "writes to Array.prototype.push, so the string decoder can't be trusted.",
    );
    expect(sentenceFor('BUILTIN_PATCHED: a prototype[a computed key]')).toBe(
      "writes to a prototype by a computed key, so the string decoder can't be trusted.",
    );
    expect(sentenceFor('reflective read (runner)')).toMatch(/^reads a property by a key/);
    expect(sentenceFor('an intrinsic passed to a user function')).toMatch(/^hands a built-in to one of the program's functions/);
    expect(sentenceFor("a folded chain's assumed key may be written")).toMatch(/^may write a key/);
    expect(sentenceFor('write by an unknown key')).toMatch(/^writes a property by a key/);
    expect(sentenceFor('something new')).toBe('something new.');
  });
});
