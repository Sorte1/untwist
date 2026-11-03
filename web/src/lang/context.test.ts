import { javascriptLanguage } from '@codemirror/lang-javascript';
import { ensureSyntaxTree, LanguageSupport } from '@codemirror/language';
import { EditorState } from '@codemirror/state';
import { describe, expect, it } from 'vitest';
import { callAt, memberNameAt, stringAt, typedAt } from './context';

function stateOf(text: string): EditorState {
  const state = EditorState.create({ doc: text, extensions: [new LanguageSupport(javascriptLanguage)] });
  ensureSyntaxTree(state, state.doc.length, 1000);
  return state;
}

function at(text: string, marker = '|'): { state: EditorState; pos: number } {
  const pos = text.indexOf(marker);
  return { state: stateOf(text.replace(marker, '')), pos };
}

describe('what the cursor is typing', () => {
  it('a member after a dot, with the object it belongs to', () => {
    const { state, pos } = at('const box = {};\nbox.|');
    expect(typedAt(state, pos)).toEqual({ kind: 'member', subject: { name: 'box', from: 16, form: undefined }, from: pos });
  });

  it('a half-typed member keeps the start of its name', () => {
    const { state, pos } = at('box.wi|');
    expect(typedAt(state, pos)).toEqual({ kind: 'member', subject: { name: 'box', from: 0, form: undefined }, from: 4 });
  });

  it('a member of a literal knows the literal form', () => {
    const { state, pos } = at("'text'.|");
    expect(typedAt(state, pos)).toMatchObject({ kind: 'member', subject: { name: null, form: 'string' } });
    expect(typedAt(...Object.values(at('[1, 2].ma|')) as [EditorState, number])).toMatchObject({ kind: 'member', subject: { form: 'array' } });
  });

  it('a plain word from its first character', () => {
    const { state, pos } = at('let total = 1;\ntot|');
    expect(typedAt(state, pos)).toEqual({ kind: 'word', from: pos - 3 });
  });

  it('no word after a digit', () => {
    const { state, pos } = at('add(1|');
    expect(typedAt(state, pos)).toEqual({ kind: 'word', from: pos });
  });

  it('nothing inside a string or a comment', () => {
    expect(typedAt(...(Object.values(at('x = "ab|c"')) as [EditorState, number]))).toEqual({ kind: 'quiet' });
    expect(typedAt(...(Object.values(at('x = 1; // no|')) as [EditorState, number]))).toEqual({ kind: 'quiet' });
    expect(typedAt(...(Object.values(at('/* a| */')) as [EditorState, number]))).toEqual({ kind: 'quiet' });
    expect(typedAt(...(Object.values(at('x = "abc"|')) as [EditorState, number]))).toEqual({ kind: 'word', from: 9 });
  });
});

describe('the call around the cursor', () => {
  it('counts the commas before the cursor at its own level', () => {
    const { state, pos } = at('add(1, f(2, 3), |)');
    expect(callAt(state, pos)).toEqual({ callee: 'add', subject: null, calleeFrom: 0, isNew: false, argIndex: 2, open: 3 });
  });

  it('finds the innermost call', () => {
    const { state, pos } = at('add(1, f(2, |))');
    expect(callAt(state, pos)).toMatchObject({ callee: 'f', argIndex: 1 });
  });

  it('reads a method on an object and a constructor', () => {
    expect(callAt(...(Object.values(at('Math.max(1, |')) as [EditorState, number]))).toMatchObject({
      callee: 'max',
      subject: { name: 'Math' },
      argIndex: 1,
    });
    expect(callAt(...(Object.values(at('new Map(|')) as [EditorState, number]))).toMatchObject({ callee: 'Map', isNew: true, argIndex: 0 });
  });

  it('is nothing outside the parentheses or inside a string argument', () => {
    expect(callAt(...(Object.values(at('add(1)|')) as [EditorState, number]))).toBeNull();
    expect(callAt(...(Object.values(at('|add(1)')) as [EditorState, number]))).toBeNull();
    expect(callAt(...(Object.values(at('add("a|")')) as [EditorState, number]))).toBeNull();
  });
});

describe('hover targets', () => {
  it('a string literal with its text', () => {
    const { state, pos } = at('x = "\\x41|B";');
    expect(stringAt(state, pos)).toEqual({ from: 4, to: 11, literal: '"\\x41B"' });
  });

  it('a property name with the object it is read from', () => {
    const { state, pos } = at('Math.fl|oor(1)');
    expect(memberNameAt(state, pos)).toEqual({ property: 'floor', subject: { name: 'Math', from: 0, form: undefined }, from: 5, to: 10 });
  });
});
