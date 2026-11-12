import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { headerPolicy, metaPolicy } from './csp';

const HEADERS = readFileSync(new URL('../public/_headers', import.meta.url), 'utf8');

describe('the content security policy', () => {
  it('is sent as a header that forbids framing, base and form targets and plugins', () => {
    const policy = headerPolicy(HEADERS);
    for (const directive of ["frame-ancestors 'none'", "base-uri 'none'", "form-action 'none'", "object-src 'none'", "script-src 'self' 'wasm-unsafe-eval'"]) {
      expect(policy).toContain(directive);
    }
  });

  it('goes into the page meta tag without the directives a meta tag ignores', () => {
    const meta = metaPolicy(HEADERS);
    expect(meta).not.toContain('frame-ancestors');
    expect(meta).toContain("base-uri 'none'");
    expect(meta).toBe(
      headerPolicy(HEADERS)
        .split('; ')
        .filter((directive) => !directive.startsWith('frame-ancestors'))
        .join('; '),
    );
  });

  it('stops the build when the headers carry no policy', () => {
    expect(() => metaPolicy('/*\n  X-Frame-Options: DENY\n')).toThrow(/no Content-Security-Policy/);
  });
});
