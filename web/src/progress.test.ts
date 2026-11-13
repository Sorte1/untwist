import { describe, expect, it } from 'vitest';
import { expectedPhases, phaseOf, progressOf, stageLine, type StageMark } from './progress';

const marks = (...names: string[]): StageMark[] => names.map((name, index) => ({ name, at: index * 100 }));

describe('phaseOf', () => {
  it('groups the engine stages into the phases shown', () => {
    expect(['prepare', 'layers'].map(phaseOf)).toEqual(['prepare', 'prepare']);
    expect(['deobfuscate', 'protections'].map(phaseOf)).toEqual(['deobfuscate', 'deobfuscate']);
    expect(['unminify', 'lexical'].map(phaseOf)).toEqual(['unminify', 'unminify']);
    expect(['rename', 'rename-shorthand', 'jsx', 'cleanup', 'unpack'].map(phaseOf)).toEqual(['rename', 'rename', 'jsx', 'cleanup', 'unpack']);
    expect(phaseOf('plugin:after-parse')).toBeNull();
  });
});

describe('expectedPhases', () => {
  it('follows the flags of the run, in the order the engine first reaches them', () => {
    expect(expectedPhases('--unsafe')).toEqual(['prepare', 'deobfuscate', 'unminify', 'jsx', 'cleanup', 'unpack']);
    expect(expectedPhases('--rename unique')).toEqual(['prepare', 'deobfuscate', 'unminify', 'jsx', 'cleanup', 'rename', 'unpack']);
    expect(expectedPhases('--no-deobfuscate --no-unminify --no-jsx --no-unpack')).toEqual(['prepare']);
    expect(expectedPhases('--rename none')).not.toContain('rename');
  });
});

describe('progressOf', () => {
  it('marks nothing before the first stage', () => {
    const progress = progressOf('--no-unpack', [], 50);
    expect(progress.steps.map((step) => step.state)).toEqual(['pending', 'pending', 'pending', 'pending', 'pending']);
    expect(progress.stage).toBeNull();
  });

  it('marks the phase of the last stage current and the ones seen before it done', () => {
    const progress = progressOf('', marks('prepare', 'deobfuscate', 'unminify'), 450);
    expect(progress.steps.map((step) => `${step.label}:${step.state}`)).toEqual(['Prepare:done', 'Deobfuscate:done', 'Unminify:current', 'JSX:pending', 'Clean up:pending', 'Unpack:pending']);
    expect(progress.stage).toBe('unminify');
    expect(progress.round).toBe(1);
    expect(progress.stageMs).toBe(250);
  });

  it('goes back to a phase the engine returns to and counts its rounds', () => {
    const progress = progressOf('', marks('prepare', 'deobfuscate', 'unminify', 'cleanup', 'unminify'), 400);
    expect(progress.steps.map((step) => step.state)).toEqual(['done', 'done', 'current', 'pending', 'done', 'pending']);
    expect(progress.round).toBe(2);
  });

  it('ignores stages outside the shown phases', () => {
    const progress = progressOf('', marks('prepare', 'plugin:after-parse'), 100);
    expect(progress.stage).toBe('prepare');
  });
});

describe('stageLine', () => {
  it('names the stage and its round', () => {
    expect(stageLine(progressOf('', marks('prepare'), 0))).toBe('Stage: prepare');
    expect(stageLine(progressOf('', marks('cleanup', 'unminify', 'cleanup'), 0))).toBe('Stage: cleanup · round 2');
    expect(stageLine(progressOf('', [], 0))).toBe('Starting the engine…');
  });

  it('says which library tables load until the engine reports a stage', () => {
    expect(stageLine(progressOf('', marks('library-tables:core-js'), 0))).toBe('Loading library tables: core-js…');
    expect(stageLine(progressOf('', marks('library-tables:core-js', 'prepare'), 0))).toBe('Stage: prepare');
  });
});
