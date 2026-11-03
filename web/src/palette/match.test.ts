import { describe, expect, it } from 'vitest';
import { filterCommands, score } from './match';

const command = (group: string, title: string) => ({ group, title, hint: '', run: () => undefined });

describe('score', () => {
  it('matches the query letters in order, ignoring case and spaces', () => {
    expect(score('web5', 'webpack 5 bundle')).not.toBeNull();
    expect(score('WEBPACK 5', 'webpack 5 bundle')).not.toBeNull();
    expect(score('5web', 'webpack 5 bundle')).toBeNull();
    expect(score('xyz', 'Run')).toBeNull();
  });

  it('ranks letters at word starts and in runs above scattered ones', () => {
    expect(score('un', 'Unsafe mode') ?? 0).toBeGreaterThan(score('un', 'Run') ?? 0);
    expect(score('rename', 'Rename: unique') ?? 0).toBeGreaterThan(score('rename', 'Run the module tree in a new editor') ?? 0);
    expect(score('run', 'Action Run') ?? 0).toBeGreaterThan(score('run', 'Rename Rename: unique') ?? 0);
    expect(score('mode', 'Mode Safe mode') ?? 0).toBeGreaterThan(score('mode', 'Module src/index.js') ?? 0);
  });

  it('gives every text a zero score for an empty query', () => {
    expect(score('', 'anything')).toBe(0);
    expect(score('   ', 'anything')).toBe(0);
  });
});

describe('filterCommands', () => {
  const list = [command('Run', 'Run'), command('Mode', 'Unsafe mode'), command('Example', 'webpack 5 bundle'), command('Setting', 'Unpack bundles')];

  it('keeps the order for an empty query', () => {
    expect(filterCommands(list, '').map((item) => item.title)).toEqual(['Run', 'Unsafe mode', 'webpack 5 bundle', 'Unpack bundles']);
  });

  it('drops what does not match and puts the best match first', () => {
    expect(filterCommands(list, 'un').map((item) => item.title)).toEqual(['Unsafe mode', 'Unpack bundles', 'Run', 'webpack 5 bundle']);
    expect(filterCommands(list, 'bundles').map((item) => item.title)).toEqual(['Unpack bundles']);
    expect(filterCommands(list, 'bundle').map((item) => item.title)).toEqual(['webpack 5 bundle', 'Unpack bundles']);
  });

  it('searches the group too', () => {
    expect(filterCommands(list, 'example web').map((item) => item.title)).toEqual(['webpack 5 bundle']);
  });

  it('keeps at most the limit', () => {
    const many = Array.from({ length: 30 }, (_, index) => command('Module', `src/m${index}.js`));
    expect(filterCommands(many, 'src', 10)).toHaveLength(10);
  });
});
