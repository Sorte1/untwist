import { describe, expect, it } from 'vitest';
import { fromInput, fromOutput, moduleRegion, pairsOf, textOffset } from './model';

const PAIRS = pairsOf(JSON.stringify([
  [0, 40, 0, 50],
  [10, 20, 12, 30],
  [21, 30, 12, 30],
  [41, 50, 60, 70],
]));

describe('the linked selection', () => {
  it('reads the pairs and skips broken ones', () => {
    expect(PAIRS).toHaveLength(4);
    expect(pairsOf(JSON.stringify([[1, 2, 3], [4, 3, 1, 2], 'x', [0, 1, 0, 1]]))).toEqual([[0, 1, 0, 1]]);
    expect(pairsOf('')).toEqual([]);
    expect(pairsOf('{')).toEqual([]);
  });

  it('links an output position to the innermost statement around it', () => {
    expect(fromOutput(PAIRS, 15)).toEqual({ output: [10, 20], input: [12, 30] });
    expect(fromOutput(PAIRS, 35)).toEqual({ output: [0, 40], input: [0, 50] });
    expect(fromOutput(PAIRS, 40)).toEqual({ output: [0, 40], input: [0, 50] });
    expect(fromOutput(PAIRS, 99)).toBeNull();
  });

  it('links an input position to every output statement printed from it', () => {
    expect(fromInput(PAIRS, 20)).toEqual({ input: [12, 30], outputs: [[10, 20], [21, 30]] });
    expect(fromInput(PAIRS, 65)).toEqual({ input: [60, 70], outputs: [[41, 50]] });
    expect(fromInput(PAIRS, 55)).toBeNull();
  });

  it('covers a module with the input of the statements printed inside it', () => {
    expect(moduleRegion(PAIRS, [5, 35])).toEqual([12, 30]);
    expect(moduleRegion(PAIRS, [0, 60])).toEqual([0, 70]);
    expect(moduleRegion(PAIRS, [90, 99])).toBeNull();
  });

  it('turns an editor position back into a text offset across CRLF line ends', () => {
    expect(textOffset('a\r\nb\r\nc', 0)).toBe(0);
    expect(textOffset('a\r\nb\r\nc', 2)).toBe(3);
    expect(textOffset('a\r\nb\r\nc', 4)).toBe(6);
    expect(textOffset('abc', 2)).toBe(2);
  });
});
