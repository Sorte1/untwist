import { javascriptLanguage } from '@codemirror/lang-javascript';
import { ensureSyntaxTree, foldable, LanguageSupport } from '@codemirror/language';
import { EditorState } from '@codemirror/state';
import { describe, expect, it } from 'vitest';
import { editorKeys } from './editor';
import { libraryPlaceholder } from './editing';

describe('the editor basics', () => {
  it('leave Ctrl+Enter to Run instead of inserting a blank line', () => {
    expect(editorKeys.map((binding) => binding.key)).not.toContain('Mod-Enter');
    expect(editorKeys.map((binding) => binding.key)).toContain('Mod-/');
  });

  it('fold a function body from the line that opens it', () => {
    const state = EditorState.create({ doc: 'function f() {\n  a();\n  b();\n}\nf();', extensions: [new LanguageSupport(javascriptLanguage)] });
    ensureSyntaxTree(state, state.doc.length, 1000);
    const first = state.doc.line(1);
    expect(foldable(state, first.from, first.to)).toEqual({ from: first.to, to: state.doc.line(4).from });
    const last = state.doc.line(5);
    expect(foldable(state, last.from, last.to)).toBeNull();
  });

  it('label a fold that starts after a library comment with its line count and library', () => {
    const state = EditorState.create({ doc: '// core-js 3.x: internals/a.js\nfunction a() {\n  b();\n}\nfunction c() {\n}' });
    const comment = state.doc.line(1);
    expect(libraryPlaceholder(state, { from: comment.to, to: state.doc.line(4).to })).toBe('3 lines of core-js');
    const body = state.doc.line(2);
    expect(libraryPlaceholder(state, { from: body.to, to: state.doc.line(4).from })).toBeNull();
  });
});
