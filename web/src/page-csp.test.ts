import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const PAGE = readFileSync(new URL('../index.html', import.meta.url), 'utf8');

describe('index.html under style-src self', () => {
  it('has no select, whose WebKit shadow stylesheet the policy refuses', () => {
    expect(PAGE).not.toMatch(/<select\b/i);
  });

  it('has no inline style attribute or element', () => {
    expect(PAGE).not.toMatch(/\sstyle=|<style\b/i);
  });
});
