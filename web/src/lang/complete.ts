import type { Completion, CompletionContext, CompletionResult } from '@codemirror/autocomplete';
import { globals, membersOf, signatureOf, type Builtin, type Owner } from './builtins';
import type { LangClient } from './client';
import { typedAt, type Subject } from './context';
import { parsed } from './parsed';
import { jsSnippets } from './snippets';

export interface EngineItem {
  readonly label: string;
  readonly kind: string;
  readonly params?: readonly string[];
}

export interface Described {
  readonly kind: string;
  readonly form?: string;
  readonly value?: string;
  readonly decoded?: boolean;
  readonly params?: readonly string[];
  readonly properties: readonly string[];
}

const IDENTIFIER = /^[\w$]*$/;
const BUILTIN_BOOST = -10;
const TOP_BOOST = 99;

const ENGINE_TYPES: Readonly<Record<string, string>> = {
  function: 'function',
  class: 'class',
  const: 'constant',
  import: 'namespace',
  keyword: 'keyword',
};

const BUILTIN_TYPES: Readonly<Record<Builtin['kind'], string>> = {
  function: 'function',
  class: 'class',
  namespace: 'namespace',
  property: 'property',
  method: 'method',
};

export function engineType(kind: string): string {
  return ENGINE_TYPES[kind] ?? 'variable';
}

function infoOf(signature: string, doc: string): () => HTMLElement {
  return () => {
    const dom = document.createElement('div');
    dom.className = 'cm-lang-info';
    const sig = document.createElement('div');
    sig.className = 'cm-lang-info-sig';
    sig.textContent = signature;
    dom.append(sig);
    if (doc !== '') {
      const text = document.createElement('div');
      text.className = 'cm-lang-info-doc';
      text.textContent = doc;
      dom.append(text);
    }
    return dom;
  };
}

export function paramsLabel(params: readonly string[] | undefined): string {
  return params === undefined ? '' : `(${params.join(', ')})`;
}

function engineOption(item: EngineItem, index: number, builtin: Builtin | undefined): Completion {
  if (item.kind === 'global' && builtin !== undefined) {
    return builtinOption(builtin, '', 0);
  }
  const detail = item.params === undefined ? item.kind : `${item.kind} ${paramsLabel(item.params)}`;
  const option: Completion = { label: item.label, type: engineType(item.kind), detail, boost: Math.max(-99, TOP_BOOST - index) };
  return item.params === undefined ? option : { ...option, info: infoOf(`${item.label}${paramsLabel(item.params)}`, '') };
}

export function builtinOption(builtin: Builtin, owner: string, boost: number): Completion {
  const params = builtin.params === null ? '' : paramsLabel(builtin.params);
  return {
    label: builtin.name,
    type: BUILTIN_TYPES[builtin.kind],
    detail: builtin.returns === '' ? params : `${params}: ${builtin.returns}`.replace(/^: /, ''),
    info: infoOf(signatureOf(builtin, owner), builtin.doc),
    boost,
  };
}

export function wordOptions(items: readonly EngineItem[]): Completion[] {
  const table = globals();
  const seen = new Set(items.map((item) => item.label));
  const scoped = items.map((item, index) => engineOption(item, index, table.get(item.label)));
  const builtins = [...table.values()].filter((builtin) => !seen.has(builtin.name)).map((builtin) => builtinOption(builtin, '', BUILTIN_BOOST));
  return [...scoped, ...builtins, ...jsSnippets];
}

export function ownerOf(subject: Subject, described: Described | null): Owner {
  return {
    name: subject.name ?? '',
    global: subject.name !== null && (described === null || described.kind === 'global'),
    form: subject.form ?? described?.form,
  };
}

export function memberOptions(subject: Subject, described: Described | null, typing: string): Completion[] {
  const owner = ownerOf(subject, described);
  const builtins = membersOf(owner);
  const documented = new Set(builtins.map((builtin) => builtin.name));
  const seen = (described?.properties ?? []).filter((name) => name !== typing && !documented.has(name) && IDENTIFIER.test(name));
  const ownerName = owner.global ? owner.name : '';
  return [
    ...seen.map((name): Completion => ({ label: name, type: 'property', detail: 'property', boost: TOP_BOOST })),
    ...builtins.map((builtin) => builtinOption(builtin, ownerName, 0)),
  ];
}

export async function describeAt(client: LangClient, pos: number, name: string): Promise<Described | null> {
  return parsed<Described>(await client.query('describe', pos, name));
}

export function completionSource(client: LangClient) {
  return async (context: CompletionContext): Promise<CompletionResult | null> => {
    const typed = typedAt(context.state, context.pos);
    if (typed.kind === 'quiet') {
      return null;
    }
    if (typed.kind === 'member') {
      const { subject } = typed;
      const described = subject.name === null ? null : await describeAt(client, subject.from, subject.name);
      const options = memberOptions(subject, described, context.state.sliceDoc(typed.from, context.pos));
      return context.aborted || options.length === 0 ? null : { from: typed.from, options, validFor: IDENTIFIER };
    }
    if (typed.from === context.pos && !context.explicit) {
      return null;
    }
    const items = parsed<EngineItem[]>(await client.query('completions', context.pos)) ?? [];
    if (context.aborted) {
      return null;
    }
    return { from: typed.from, options: wordOptions(items), validFor: IDENTIFIER };
  };
}
