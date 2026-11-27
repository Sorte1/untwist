const MAGIC = 'UTBLOOM1';
const BITS_PER_KEY = 10n;
const HASHES = 7;
const SEED = 0x5bd1_e995_9e37_79b9n;
const MIN_BITS = 64n;
const MASK = (1n << 64n) - 1n;
const LOW = 0xffff_ffffn;

export interface Bloom {
  readonly bits: Uint8Array<ArrayBuffer>;
  readonly count: bigint;
  readonly hashes: number;
}

export function mix(value: bigint): bigint {
  let z = (value + 0x9e37_79b9_7f4a_7c15n) & MASK;
  z = ((z ^ (z >> 30n)) * 0xbf58_476d_1ce4_e5b9n) & MASK;
  z = ((z ^ (z >> 27n)) * 0x94d0_49bb_1331_11ebn) & MASK;
  return z ^ (z >> 31n);
}

function positions(key: number, count: bigint, hashes: number): bigint[] {
  const hash = mix(BigInt(key) ^ SEED);
  const first = hash & LOW;
  const step = (hash >> 32n) | 1n;
  return Array.from({ length: hashes }, (_, index) => ((first + BigInt(index) * step) & MASK) % count);
}

export function bloomWithKeys(keys: Iterable<number>): Bloom {
  const unique = [...new Set(keys)].sort((a, b) => a - b);
  const wanted = BigInt(unique.length) * BITS_PER_KEY;
  const least = wanted > MIN_BITS ? wanted : MIN_BITS;
  const count = ((least + 7n) / 8n) * 8n;
  const bits = new Uint8Array(Number(count / 8n));
  for (const key of unique) {
    for (const position of positions(key, count, HASHES)) {
      const byte = Number(position / 8n);
      bits[byte] = (bits[byte] ?? 0) | (1 << Number(position % 8n));
    }
  }
  return { bits, count, hashes: HASHES };
}

export function bloomFile(entries: readonly (readonly [string, Bloom])[]): Uint8Array<ArrayBuffer> {
  const encoder = new TextEncoder();
  const ids = entries.map(([id]) => encoder.encode(id));
  const size = MAGIC.length + 4 + entries.reduce((sum, [, bloom], at) => sum + 4 + (ids[at]?.length ?? 0) + 1 + 8 + bloom.bits.length, 0);
  const out = new Uint8Array(size);
  const view = new DataView(out.buffer);
  out.set(encoder.encode(MAGIC), 0);
  let at = MAGIC.length;
  view.setUint32(at, entries.length, true);
  at += 4;
  entries.forEach(([, bloom], index) => {
    const id = ids[index] ?? new Uint8Array();
    view.setUint32(at, id.length, true);
    out.set(id, at + 4);
    at += 4 + id.length;
    out[at] = bloom.hashes;
    view.setBigUint64(at + 1, bloom.count, true);
    out.set(bloom.bits, at + 9);
    at += 9 + bloom.bits.length;
  });
  return out;
}
