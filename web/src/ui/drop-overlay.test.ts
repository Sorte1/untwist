import { describe, expect, it } from 'vitest';
import { EMPTY_DROP_NOTE, loadDropped, mayCarryFiles } from './drop-overlay';

describe('mayCarryFiles', () => {
  it('accepts a drag that says it carries files', () => {
    expect(mayCarryFiles(['Files'])).toBe(true);
    expect(mayCarryFiles(['application/x-moz-file', 'Files'])).toBe(true);
  });

  it('accepts a drag whose types are hidden until the drop, as Firefox on Wayland reports them', () => {
    expect(mayCarryFiles([])).toBe(true);
  });

  it('leaves text dragged inside the page alone', () => {
    expect(mayCarryFiles(['text/plain'])).toBe(false);
    expect(mayCarryFiles(['text/plain', 'text/html'])).toBe(false);
  });
});

describe('loadDropped', () => {
  it('notes a drop that carried no file', async () => {
    const notes: string[] = [];
    await loadDropped([], { load: () => 0, open: () => 0, note: (text) => notes.push(text) });
    expect(notes).toEqual([EMPTY_DROP_NOTE]);
  });
});
