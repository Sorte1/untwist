import { describe, expect, it } from 'vitest';
import {
  attemptLine,
  disabledPasses,
  durationLabel,
  expectedPhases,
  librariesOn,
  MARK,
  phaseOf,
  progressOf,
  stageLine,
  tablesMark,
  timelineRows,
  type StageMark,
} from './progress';

const marks = (...names: string[]): StageMark[] => names.map((name, index) => ({ name, at: index * 100 }));
const OFF = '--disable library-names,library-rename';
const shown = (flags: string, list: StageMark[], now: number) => progressOf(flags, list, now).steps.map((step) => `${step.phase}:${step.state}:${step.ms}`);

describe('phaseOf', () => {
  it('groups the engine stages into the phases shown', () => {
    expect(['prepare', 'layers'].map(phaseOf)).toEqual(['prepare', 'prepare']);
    expect(['deobfuscate', 'protections'].map(phaseOf)).toEqual(['deobfuscate', 'deobfuscate']);
    expect(['unminify', 'lexical'].map(phaseOf)).toEqual(['unminify', 'unminify']);
    expect(['rename', 'rename-shorthand', 'jsx', 'cleanup', 'unpack'].map(phaseOf)).toEqual(['rename', 'rename', 'jsx', 'cleanup', 'unpack']);
    expect(['parse', 'print', 'library', 'modern'].map(phaseOf)).toEqual(['parse', 'print', 'library', 'modern']);
    expect(phaseOf('plugin:after-parse')).toBeNull();
  });

  it('places the playground steps', () => {
    expect([MARK.scan, MARK.loadTables, MARK.scanOutput, 'tables:core-js (2.1 MB)'].map(phaseOf)).toEqual(['tables', 'tables', 'tables', 'tables']);
    expect([MARK.start, MARK.retry, MARK.rerun].map(phaseOf)).toEqual(['start', 'start', 'start']);
    expect([MARK.transfer, MARK.editor, MARK.folds].map(phaseOf)).toEqual(['transfer', 'editor', 'editor']);
  });
});

describe('librariesOn', () => {
  it('reads the passes the flags turn off', () => {
    expect(disabledPasses('--unsafe --disable dead-code,library-names --rename smart')).toEqual(['dead-code', 'library-names']);
    expect(librariesOn('--unsafe')).toBe(true);
    expect(librariesOn(OFF)).toBe(false);
    expect(librariesOn('--disable library-rename')).toBe(true);
  });
});

describe('expectedPhases', () => {
  it('follows the flags of the run, in the order the engine first reaches them', () => {
    expect(expectedPhases(`--unsafe ${OFF}`)).toEqual(['start', 'parse', 'prepare', 'deobfuscate', 'unminify', 'jsx', 'cleanup', 'print', 'unpack', 'transfer']);
    expect(expectedPhases(`--rename unique ${OFF}`)).toContain('rename');
    expect(expectedPhases(`--no-deobfuscate --no-unminify --no-jsx --no-unpack ${OFF}`)).toEqual(['start', 'parse', 'prepare', 'print', 'transfer']);
    expect(expectedPhases('--rename none')).not.toContain('rename');
  });

  it('shows the library steps only when library names are on, and the editor only when asked', () => {
    expect(expectedPhases('')).toEqual(['tables', 'start', 'parse', 'prepare', 'deobfuscate', 'unminify', 'jsx', 'cleanup', 'library', 'print', 'unpack', 'transfer']);
    expect(expectedPhases(OFF, { libraries: false, editor: true }).at(-1)).toBe('editor');
  });
});

describe('progressOf', () => {
  it('marks nothing before the first mark', () => {
    const progress = progressOf(`--no-unpack ${OFF}`, [], 50);
    expect(progress.steps.every((step) => step.state === 'pending' && step.ms === 0)).toBe(true);
    expect(progress.stage).toBeNull();
    expect(progress.attempt).toBe('first');
  });

  it('times each phase from its mark to the next one, and the current one up to now', () => {
    expect(shown(`--no-jsx --no-unpack --no-deobfuscate ${OFF}`, marks('start', 'parse', 'prepare', 'unminify'), 450)).toEqual([
      'start:done:100',
      'parse:done:100',
      'prepare:done:100',
      'unminify:current:150',
      'print:pending:0',
      'transfer:pending:0',
    ]);
  });

  it('adds up the rounds of a phase the engine returns to and counts the rounds of the stage', () => {
    const progress = progressOf(OFF, marks('parse', 'deobfuscate', 'unminify', 'protections', 'cleanup', 'protections'), 700);
    const deob = progress.steps.find((step) => step.phase === 'deobfuscate');
    expect(deob).toMatchObject({ state: 'current', ms: 400 });
    expect(progress.stage).toBe('protections');
    expect(progress.round).toBe(2);
    expect(progress.stageMs).toBe(200);
  });

  it('gives the time of an unknown stage to the phase before it', () => {
    const progress = progressOf(OFF, marks('prepare', 'plugin:after-prepare', 'deobfuscate'), 300);
    expect(progress.steps.find((step) => step.phase === 'prepare')?.ms).toBe(200);
  });

  it('sorts marks that arrive out of order by their time', () => {
    const list = [
      { name: 'prepare', at: 200 },
      { name: 'parse', at: 100 },
    ];
    expect(progressOf(OFF, list, 250).steps.find((step) => step.phase === 'parse')).toMatchObject({ state: 'done', ms: 100 });
  });

  it('shows a phase it did not expect once the engine reaches it', () => {
    expect(progressOf(OFF, marks('parse', 'modern'), 300).steps.map((step) => step.phase)).toContain('modern');
  });

  it('starts the engine phases over on a depth retry and keeps the time of the failed attempt apart', () => {
    const progress = progressOf(OFF, marks('start', 'parse', 'deobfuscate', MARK.retry, 'start', 'parse'), 650);
    expect(progress.attempt).toBe('retry');
    expect(progress.earlierMs).toBe(300);
    expect(progress.steps.find((step) => step.phase === 'deobfuscate')?.state).toBe('pending');
    expect(progress.steps.find((step) => step.phase === 'parse')).toMatchObject({ state: 'current', ms: 150 });
    expect(attemptLine(progress)).toBe('Retrying (depth limit): the first attempt hit the stack limit');
  });

  it('keeps the library steps across a rerun with the tables found in the output', () => {
    const list = marks(MARK.scan, 'start', 'parse', MARK.transfer, MARK.scanOutput, 'tables:core-js (2.1 MB)', MARK.rerun, 'start');
    const progress = progressOf('', list, 800);
    expect(progress.attempt).toBe('rerun');
    expect(progress.steps.find((step) => step.phase === 'tables')).toMatchObject({ state: 'done', ms: 300 });
    expect(attemptLine(progress)).toBe('Running again with the library tables found in the output');
  });

  it('ends with no current step once the end mark is in', () => {
    const progress = progressOf(OFF, marks('parse', MARK.editor, MARK.end), 900);
    expect(progress.stage).toBeNull();
    expect(progress.steps.some((step) => step.state === 'current')).toBe(false);
    expect(progress.steps.find((step) => step.phase === 'editor')?.ms).toBe(100);
  });
});

describe('stageLine', () => {
  it('names the stage and its round', () => {
    expect(stageLine(progressOf('', marks('prepare'), 0))).toBe('Stage: prepare');
    expect(stageLine(progressOf('', marks('cleanup', 'unminify', 'cleanup'), 0))).toBe('Stage: cleanup · round 2');
    expect(stageLine(progressOf('', [], 0))).toBe('Starting the engine…');
  });

  it('describes the playground steps', () => {
    expect(stageLine(progressOf('', marks(MARK.scan), 0))).toBe('Scanning the input for library code…');
    expect(stageLine(progressOf('', marks(tablesMark([{ label: 'core-js', bytes: 2_202_009 }], (bytes) => `${(bytes / 1048576).toFixed(1)} MB`)), 0))).toBe('Fetching library tables: core-js (2.1 MB)…');
    expect(stageLine(progressOf('', marks(MARK.start), 0))).toBe('Starting the engine…');
    expect(stageLine(progressOf('', marks(MARK.transfer), 0))).toBe('Transferring the output…');
    expect(stageLine(progressOf('', marks(MARK.editor), 0))).toBe('Loading the output into the editor…');
  });
});

describe('timelineRows', () => {
  it('lists the phases reached with their time, and the failed attempt first', () => {
    const progress = progressOf(OFF, marks('start', 'parse', MARK.retry, 'start', 'parse', MARK.end), 600);
    expect(timelineRows(progress)).toEqual([
      { label: 'Attempt that hit the depth limit', value: '200 ms' },
      { label: 'Start worker', value: '200 ms' },
      { label: 'Parse', value: '100 ms' },
    ]);
  });

  it('writes short times in ms and long ones in seconds', () => {
    expect(durationLabel(12.4)).toBe('12 ms');
    expect(durationLabel(2345)).toBe('2.35 s');
  });
});
