import { syntaxTree } from '@codemirror/language';
import type { EditorState } from '@codemirror/state';
import type { SyntaxNode } from '@lezer/common';

const QUIET = new Set(['String', 'TemplateString', 'LineComment', 'BlockComment', 'RegExp']);
const LITERAL_FORMS: Readonly<Record<string, string>> = {
  String: 'string',
  TemplateString: 'string',
  ArrayExpression: 'array',
  Number: 'number',
  RegExp: 'regexp',
  ObjectExpression: 'object',
  ArrowFunction: 'function',
  FunctionExpression: 'function',
};
const WORD = /[A-Za-z_$][\w$]*$/;

export interface Subject {
  readonly name: string | null;
  readonly from: number;
  readonly form: string | undefined;
}

export type Typed =
  | { readonly kind: 'member'; readonly subject: Subject; readonly from: number }
  | { readonly kind: 'word'; readonly from: number }
  | { readonly kind: 'quiet' };

export interface Call {
  readonly callee: string;
  readonly subject: Subject | null;
  readonly calleeFrom: number;
  readonly isNew: boolean;
  readonly argIndex: number;
  readonly open: number;
}

function text(state: EditorState, node: SyntaxNode): string {
  return state.sliceDoc(node.from, node.to);
}

function enclosing(node: SyntaxNode | null, names: ReadonlySet<string>): SyntaxNode | null {
  for (let at = node; at !== null; at = at.parent) {
    if (names.has(at.name)) {
      return at;
    }
  }
  return null;
}

function closedString(state: EditorState, node: SyntaxNode): boolean {
  const body = text(state, node);
  return body.length >= 2 && body[body.length - 1] === body[0];
}

function isQuiet(state: EditorState, node: SyntaxNode, pos: number): boolean {
  const quiet = enclosing(node, QUIET);
  if (quiet === null || pos <= quiet.from) {
    return false;
  }
  if (pos < quiet.to || quiet.name === 'LineComment') {
    return true;
  }
  return (quiet.name === 'String' || quiet.name === 'TemplateString') && !closedString(state, quiet);
}

export function subjectOf(state: EditorState, node: SyntaxNode | null): Subject | null {
  if (node === null) {
    return null;
  }
  if (node.name === 'VariableName') {
    return { name: text(state, node), from: node.from, form: undefined };
  }
  const form = LITERAL_FORMS[node.name];
  return form === undefined ? null : { name: null, from: node.from, form };
}

function memberAt(state: EditorState, node: SyntaxNode, pos: number): Typed | null {
  const dot = node.name === '.' ? node : node.name === '⚠' && node.prevSibling?.name === '.' ? node.prevSibling : null;
  const property = node.name === 'PropertyName' ? node : null;
  const member = (dot ?? property)?.parent;
  if (member?.name !== 'MemberExpression') {
    return null;
  }
  const subject = subjectOf(state, member.firstChild);
  if (subject === null) {
    return { kind: 'quiet' };
  }
  return { kind: 'member', subject, from: property?.from ?? pos };
}

export function typedAt(state: EditorState, pos: number): Typed {
  const node = syntaxTree(state).resolveInner(pos, -1);
  if (isQuiet(state, node, pos)) {
    return { kind: 'quiet' };
  }
  const member = memberAt(state, node, pos);
  if (member !== null) {
    return member;
  }
  const line = state.doc.lineAt(pos);
  const word = WORD.exec(state.sliceDoc(line.from, pos));
  return { kind: 'word', from: pos - (word?.[0].length ?? 0) };
}

function argListAt(node: SyntaxNode | null, pos: number): SyntaxNode | null {
  for (let at = node; at !== null; at = at.parent) {
    if (at.name === 'ArgList' && pos > at.from && (pos < at.to || at.lastChild?.name !== ')')) {
      return at;
    }
  }
  return null;
}

function calleeOf(state: EditorState, call: SyntaxNode): Pick<Call, 'callee' | 'subject' | 'calleeFrom' | 'isNew'> | null {
  const isNew = call.name === 'NewExpression';
  const callee = isNew ? call.firstChild?.nextSibling : call.firstChild;
  if (callee === null || callee === undefined) {
    return null;
  }
  if (callee.name === 'VariableName') {
    return { callee: text(state, callee), subject: null, calleeFrom: callee.from, isNew };
  }
  const property = callee.lastChild;
  if (callee.name !== 'MemberExpression' || property?.name !== 'PropertyName') {
    return null;
  }
  return { callee: text(state, property), subject: subjectOf(state, callee.firstChild), calleeFrom: property.from, isNew };
}

export function callAt(state: EditorState, pos: number): Call | null {
  const node = syntaxTree(state).resolveInner(pos, -1);
  if (isQuiet(state, node, pos)) {
    return null;
  }
  const args = argListAt(node, pos);
  const call = args?.parent;
  if (args === null || call === null || call === undefined || (call.name !== 'CallExpression' && call.name !== 'NewExpression')) {
    return null;
  }
  const callee = calleeOf(state, call);
  if (callee === null) {
    return null;
  }
  let argIndex = 0;
  for (let child = args.firstChild; child !== null && child.to <= pos; child = child.nextSibling) {
    argIndex += child.name === ',' ? 1 : 0;
  }
  return { ...callee, argIndex, open: args.from };
}

export function stringAt(state: EditorState, pos: number): { readonly from: number; readonly to: number; readonly literal: string } | null {
  const node = enclosing(syntaxTree(state).resolveInner(pos, 1), new Set(['String']));
  if (node === null || !closedString(state, node)) {
    return null;
  }
  return { from: node.from, to: node.to, literal: text(state, node) };
}

export function memberNameAt(state: EditorState, pos: number): { readonly property: string; readonly subject: Subject; readonly from: number; readonly to: number } | null {
  const node = syntaxTree(state).resolveInner(pos, 1);
  if (node.name !== 'PropertyName' || node.parent?.name !== 'MemberExpression') {
    return null;
  }
  const subject = subjectOf(state, node.parent.firstChild);
  return subject === null ? null : { property: text(state, node), subject, from: node.from, to: node.to };
}
