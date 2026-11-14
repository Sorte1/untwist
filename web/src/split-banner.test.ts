import { describe, expect, it } from 'vitest';
import { splitsWhenAssumed } from './split-banner';

describe('splitsWhenAssumed', () => {
  it('reads the engine hint from the report and nothing else', () => {
    expect(splitsWhenAssumed('{"detection":{"bundle":null,"assumedSplit":true}}')).toBe(true);
    expect(splitsWhenAssumed('{"detection":{"bundle":null}}')).toBe(false);
    expect(splitsWhenAssumed('not json')).toBe(false);
  });
});
