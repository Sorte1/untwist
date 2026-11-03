import { describe, expect, it } from 'vitest';
import { isMac, keyLabel, SHORTCUTS, shownChords } from './shortcuts';

describe('the shortcut sheet', () => {
  it('spells chords with the platform modifier', () => {
    expect(keyLabel('Shift-Alt-f', false)).toEqual([['Shift', 'Alt', 'F']]);
    expect(keyLabel('Shift-Mod-k', true)).toEqual([['⇧', '⌘', 'K']]);
    expect(keyLabel('Mod-Enter', false)).toEqual([['Ctrl', '↵']]);
    expect(keyLabel('Mod-/', false)).toEqual([['Ctrl', '/']]);
  });

  it('repeats the modifiers of the first chord for a bare alternative', () => {
    expect(keyLabel('Alt-↑|↓', false)).toEqual([['Alt', '↑'], ['Alt', '↓']]);
    expect(keyLabel('F3|Shift-F3', false)).toEqual([['F3'], ['Shift', 'F3']]);
    expect(keyLabel('Mod-]|[', true)).toEqual([['⌘', ']'], ['⌘', '[']]);
  });

  it('names every action once', () => {
    const actions = SHORTCUTS.flatMap((group) => group.shortcuts.map((shortcut) => shortcut.action));
    expect(new Set(actions).size).toBe(actions.length);
    expect(actions).toContain('Format document');
  });

  it('tells a Mac by its platform string', () => {
    expect(isMac('MacIntel')).toBe(true);
    expect(isMac('iPhone')).toBe(true);
    expect(isMac('Linux x86_64')).toBe(false);
  });
});

describe('the shown chords', () => {
  it('drop the modifiers an alternative shares with the first chord', () => {
    expect(shownChords([['Alt', '↑'], ['Alt', '↓']])).toEqual([['Alt', '↑'], ['↓']]);
    expect(shownChords([['F3'], ['Shift', 'F3']])).toEqual([['F3'], ['Shift', 'F3']]);
    expect(shownChords([['Ctrl', 'Z'], ['Shift', 'Ctrl', 'Z']])).toEqual([['Ctrl', 'Z'], ['Shift', 'Ctrl', 'Z']]);
  });
});
