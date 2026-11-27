const MAGIC = 'UTBLOOM1';
const MASK = (1n << 64n) - 1n;
const LOW = 0xffff_ffffn;

export interface BloomParams {
  readonly bitsPerKey: bigint;
  readonly hashes: number;
  readonly seed: bigint;
  readonly minBits: bigint;
}

function rustConst(source: string, name: string): bigint {
  const found = new RegExp(`const ${name}: (?:u8|u64) = ([0-9a-fx_]+);`).exec(source)?.[1];
  if (found === undefined) {
    throw new Error(`bloom: the Rust source has no ${name}`);
  }
  return BigInt(found.replaceAll('_', ''));
}

export function bloomParams(collectionRs: string): BloomParams {
  return {
    bitsPerKey: rustConst(collectionRs, 'BLOOM_BITS_PER_KEY'),
    hashes: Number(rustConst(collectionRs, 'BLOOM_HASHES')),
    seed: rustConst(collectionRs, 'BLOOM_SEED'),
    minBits: rustConst(collectionRs, 'BLOOM_MIN_BITS'),
  };
}

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

function positions(key: number, count: bigint, hashes: number, seed: bigint): bigint[] {
  const hash = mix(BigInt(key) ^ seed);
  const first = hash & LOW;
  const step = (hash >> 32n) | 1n;
  return Array.from({ length: hashes }, (_, index) => ((first + BigInt(index) * step) & MASK) % count);
}

export function bloomWithKeys(keys: Iterable<number>, params: BloomParams): Bloom {
  const unique = [...new Set(keys)].sort((a, b) => a - b);
  const wanted = BigInt(unique.length) * params.bitsPerKey;
  const least = wanted > params.minBits ? wanted : params.minBits;
  const count = ((least + 7n) / 8n) * 8n;
  const bits = new Uint8Array(Number(count / 8n));
  for (const key of unique) {
    for (const position of positions(key, count, params.hashes, params.seed)) {
      const byte = Number(position / 8n);
      bits[byte] = (bits[byte] ?? 0) | (1 << Number(position % 8n));
    }
  }
  return { bits, count, hashes: params.hashes };
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
