import { describe, expect, it } from 'vitest';
import { join } from 'node:path';
import {
  copies,
  current,
  FILES_DIR,
  entries,
  MANIFEST_PATH,
  MAX_EXAMPLE_BYTES,
  MAX_EXAMPLES,
  MODULE_PATH,
  PICKS,
  renderManifest,
  renderModule,
  repoText,
} from '../../scripts/examples.ts';

describe('the examples', () => {
  it('are generated from the current fixtures', () => {
    const list = entries();
    expect(current(MANIFEST_PATH)).toBe(renderManifest(list));
    expect(current(MODULE_PATH)).toBe(renderModule(list));
    for (const [name, path] of copies(list)) {
      expect(current(join(FILES_DIR, name)), name).toBe(repoText(path));
    }
  });

  it('stay within the count and size caps', () => {
    const list = entries();
    expect(list.length).toBeLessThanOrEqual(MAX_EXAMPLES);
    expect(list.every((entry) => entry.bytes <= MAX_EXAMPLE_BYTES)).toBe(true);
  });

  it('refuse a fixture over the size cap', () => {
    const big = { name: 'high', source: 'fixtures/reference/deobfuscate/obfuscator.io-high.js', licence: null };
    expect(() => entries([big])).toThrow(/over/);
    expect(() => entries([...PICKS, big])).toThrow(/more than/);
  });
});
