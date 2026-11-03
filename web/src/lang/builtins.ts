import { GLOBALS, NAMESPACES, PROTOTYPES } from './builtin-data';

export type BuiltinKind = 'function' | 'class' | 'namespace' | 'property' | 'method';

export interface Builtin {
  readonly name: string;
  readonly kind: BuiltinKind;
  readonly params: readonly string[] | null;
  readonly returns: string;
  readonly doc: string;
}

const LINE = /^(new )?([\w$]+)(?:\((.*)\))?(?::\s*(.+?))?\s*\|\s*(.+)$/;
const OPENERS = new Set(['(', '[', '{']);
const CLOSERS = new Set([')', ']', '}']);

const FORM_PROTOTYPES: Readonly<Record<string, string>> = {
  string: 'string',
  array: 'array',
  number: 'number',
  regexp: 'regexp',
  function: 'function',
  class: 'function',
  'new Map': 'Map',
  'new Set': 'Set',
  'new Promise': 'Promise',
  'new Date': 'Date',
  'new Error': 'Error',
  'new TypeError': 'Error',
  'new RangeError': 'Error',
  'new SyntaxError': 'Error',
  'new RegExp': 'regexp',
  'new Array': 'array',
  'new String': 'string',
  'new Function': 'function',
};

const OBJECT_FORMS = new Set(['object', 'boolean', 'new Object']);

export function splitParams(text: string): string[] {
  const params: string[] = [];
  let depth = 0;
  let current = '';
  for (const char of text) {
    if (char === ',' && depth === 0) {
      params.push(current.trim());
      current = '';
      continue;
    }
    depth += OPENERS.has(char) ? 1 : CLOSERS.has(char) ? -1 : 0;
    current += char;
  }
  params.push(current.trim());
  return params.filter((param) => param !== '');
}

function kindOf(isNew: boolean, callable: boolean, typed: boolean, member: boolean): BuiltinKind {
  if (isNew) {
    return 'class';
  }
  if (callable) {
    return member ? 'method' : 'function';
  }
  return typed ? 'property' : 'namespace';
}

export function parseBuiltins(block: string, member: boolean): Builtin[] {
  return block
    .split('\n')
    .map((line) => LINE.exec(line.trim()))
    .filter((match): match is RegExpExecArray => match !== null)
    .map(([, isNew, name, params, returns, doc]) => ({
      name: name ?? '',
      kind: kindOf(isNew !== undefined, params !== undefined, returns !== undefined, member),
      params: params === undefined ? null : splitParams(params),
      returns: returns ?? '',
      doc: doc ?? '',
    }));
}

export function signatureOf(builtin: Builtin, owner = ''): string {
  const name = owner === '' ? builtin.name : `${owner}.${builtin.name}`;
  const prefix = builtin.kind === 'class' ? 'new ' : '';
  const params = builtin.params === null ? '' : `(${builtin.params.join(', ')})`;
  const returns = builtin.returns === '' ? '' : `: ${builtin.returns}`;
  return `${prefix}${name}${params}${returns}`;
}

function byName(list: readonly Builtin[]): ReadonlyMap<string, Builtin> {
  return new Map(list.map((builtin) => [builtin.name, builtin]));
}

let globalTable: ReadonlyMap<string, Builtin> | null = null;
const memberTables = new Map<string, readonly Builtin[]>();

export function globals(): ReadonlyMap<string, Builtin> {
  globalTable ??= byName(parseBuiltins(GLOBALS, false));
  return globalTable;
}

function table(source: Readonly<Record<string, string>>, prefix: string, key: string): readonly Builtin[] {
  const block = source[key];
  if (block === undefined) {
    return [];
  }
  const cacheKey = `${prefix}:${key}`;
  const known = memberTables.get(cacheKey);
  if (known !== undefined) {
    return known;
  }
  const parsed = parseBuiltins(block, true);
  memberTables.set(cacheKey, parsed);
  return parsed;
}

export function namespaceMembers(name: string): readonly Builtin[] {
  return table(NAMESPACES, 'ns', name);
}

export function prototypeMembers(form: string | undefined): readonly Builtin[] {
  if (form === undefined) {
    return [];
  }
  const own = FORM_PROTOTYPES[form];
  if (own === undefined && !OBJECT_FORMS.has(form)) {
    return [];
  }
  const members = own === undefined ? [] : table(PROTOTYPES, 'proto', own);
  const seen = new Set(members.map((member) => member.name));
  return [...members, ...table(PROTOTYPES, 'proto', 'object').filter((member) => !seen.has(member.name))];
}

export interface Owner {
  readonly name: string;
  readonly global: boolean;
  readonly form: string | undefined;
}

export function membersOf(owner: Owner): readonly Builtin[] {
  return owner.global ? namespaceMembers(owner.name) : prototypeMembers(owner.form);
}

export function memberBuiltin(owner: Owner, property: string): Builtin | null {
  return membersOf(owner).find((member) => member.name === property) ?? null;
}
