export const LINK_CODE_CAP = 32_768;
export const LINK_INPUT_CAP = 2 * 1024 * 1024;
export const SHARED_KEYS: readonly string[] = ['code', 'name'];
export const SHARED_NAME = 'untitled.js';

const NAME_CAP = 255;

export interface SharedInput {
  readonly code: string;
  readonly name: string | null;
}

export interface LinkInput {
  readonly text: string;
  readonly bytes: number;
  readonly name: string;
}

export type LinkPlan =
  | { readonly kind: 'code'; readonly hash: string; readonly chars: number }
  | { readonly kind: 'settings'; readonly hash: string; readonly reason: 'empty' | 'large' };

function decoded(raw: string): string {
  try {
    return decodeURIComponent(raw);
  } catch {
    return raw;
  }
}

function fields(hash: string): Map<string, string> {
  const body = hash.startsWith('#') ? hash.slice(1) : hash;
  const found = new Map<string, string>();
  for (const part of body.split('&')) {
    const split = part.indexOf('=');
    const key = decoded(split === -1 ? part : part.slice(0, split));
    if (SHARED_KEYS.includes(key) && !found.has(key)) {
      found.set(key, decoded(split === -1 ? '' : part.slice(split + 1)));
    }
  }
  return found;
}

export function sharedInputOf(hash: string): SharedInput | null {
  const found = fields(hash);
  const code = found.get('code');
  if (code === undefined) {
    return null;
  }
  const name = found.get('name');
  return { code, name: name === undefined || name === '' ? null : name.slice(0, NAME_CAP) };
}

export function shareHash(settingsHash: string, packed: string | null, name: string): string {
  if (packed === null) {
    return settingsHash;
  }
  const parts = [`code=${packed}`, ...(name === SHARED_NAME ? [] : [`name=${encodeURIComponent(name)}`])];
  return settingsHash === '' ? `#${parts.join('&')}` : `${settingsHash}&${parts.join('&')}`;
}

export function planLink(settingsHash: string, input: LinkInput, pack: (text: string) => string): LinkPlan {
  if (input.text === '') {
    return { kind: 'settings', hash: settingsHash, reason: 'empty' };
  }
  if (input.bytes > LINK_INPUT_CAP) {
    return { kind: 'settings', hash: settingsHash, reason: 'large' };
  }
  const packed = pack(input.text);
  if (packed.length > LINK_CODE_CAP) {
    return { kind: 'settings', hash: settingsHash, reason: 'large' };
  }
  return { kind: 'code', hash: shareHash(settingsHash, packed, input.name), chars: packed.length };
}
