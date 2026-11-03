import { deflateSync } from 'fflate';
import { describe, expect, it } from 'vitest';
import { packCode, SHARED_TEXT_CAP, unpackCode } from './codec';

function generated(lines: number): string {
  return Array.from({ length: lines }, (_, index) => `var v${index} = "${(index * 2654435761) % 1000003}";`).join('\n');
}

describe('packCode and unpackCode', () => {
  it('round-trip plain, empty, non-ASCII and long inputs', () => {
    for (const text of ['var a = 1;', '', 'const s = "héllo ✓ 😀";\n', generated(5000)]) {
      expect(unpackCode(packCode(text))).toBe(text);
    }
  });

  it('use only URL-safe base64 characters and give the same text for the same input', () => {
    const packed = packCode(generated(200));
    expect(packed).toMatch(/^[A-Za-z0-9_-]+$/);
    expect(packCode(generated(200))).toBe(packed);
  });

  it('compress repetitive code well below its size', () => {
    const text = generated(2000);
    expect(packCode(text).length).toBeLessThan(text.length / 2);
  });

  it('refuse characters outside base64url, data that is not deflate, and bytes that are not UTF-8', () => {
    expect(() => unpackCode('a+b/c')).toThrow(/could not be read/);
    expect(() => unpackCode('')).toThrow(/could not be read/);
    expect(() => unpackCode('_____')).toThrow(/could not be read/);
    const notUtf8 = Buffer.from(deflateSync(new Uint8Array([0x76, 0xff, 0xfe]))).toString('base64url');
    expect(() => unpackCode(notUtf8)).toThrow(/could not be read/);
  });

  it('refuse a link that inflates past the cap', () => {
    const packed = packCode('a'.repeat(SHARED_TEXT_CAP + 1));
    expect(packed.length).toBeLessThan(20_000);
    expect(() => unpackCode(packed)).toThrow(/larger than/);
  });
});
