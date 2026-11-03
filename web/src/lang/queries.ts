import type { EditorState } from '@codemirror/state';
import type { LangClient } from './client';
import { parsed } from './parsed';

export interface Span {
  readonly from: number;
  readonly to: number;
}

export interface DefinitionReply {
  readonly span: readonly [number, number];
  readonly kind: string;
  readonly module?: string;
  readonly name?: string;
  readonly form?: string;
  readonly value?: string;
  readonly decoded?: boolean;
  readonly params?: readonly string[];
}

export async function definitionAt(client: LangClient, pos: number): Promise<DefinitionReply | null> {
  return parsed<DefinitionReply>(await client.query('definition', pos));
}

export function spansOf(value: unknown): Span[] {
  if (!(value instanceof Uint32Array)) {
    return [];
  }
  const spans: Span[] = [];
  for (let index = 0; index + 1 < value.length; index += 2) {
    spans.push({ from: value[index] ?? 0, to: value[index + 1] ?? 0 });
  }
  return spans;
}

export async function referencesAt(client: LangClient, pos: number): Promise<Span[]> {
  return spansOf(await client.query('references', pos));
}

export function within(state: EditorState, from: number, to: number): Span {
  const length = state.doc.length;
  const start = Math.min(Math.max(from, 0), length);
  return { from: start, to: Math.min(Math.max(to, start), length) };
}
