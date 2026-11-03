import { describe, expect, it } from 'vitest';
import { DEFAULTS, type Settings } from '../settings/settings';
import { exampleCommands, moduleCommands, settingCommands } from './commands';

function applied(settings: Settings): { patches: Partial<Settings>[]; apply: (patch: Partial<Settings>) => void } {
  const patches: Partial<Settings>[] = [];
  return { patches, apply: (patch) => patches.push(patch) };
}

describe('settingCommands', () => {
  it('offers the mode, each switch with its state, and each rename mode', () => {
    const { apply } = applied(DEFAULTS);
    const list = settingCommands(DEFAULTS, apply);
    const rows = list.map((item) => `${item.group}|${item.title}|${item.hint}`);
    expect(rows).toEqual([
      'Mode|Safe mode|',
      'Mode|Unsafe mode|current',
      'Setting|Unpack bundles|on',
      'Setting|Decompile JSX|on',
      'Setting|Deobfuscate|on',
      'Setting|Unminify|on',
      'Setting|Keep human names|off',
      'Setting|Single file|off',
      'Setting|No fuel limit|off',
      'Rename|Rename: none|current',
      'Rename|Rename: simple|',
      'Rename|Rename: unique|',
      'Rename|Rename: mangle|',
    ]);
  });

  it('flips a switch, picks a mode and a rename mode through the patch', () => {
    const { patches, apply } = applied(DEFAULTS);
    const list = settingCommands({ ...DEFAULTS, fuel: null }, apply);
    for (const title of ['Safe mode', 'Unpack bundles', 'Deobfuscate', 'Keep human names', 'No fuel limit', 'Rename: unique']) {
      list.find((item) => item.title === title)?.run();
    }
    expect(patches).toEqual([{ unsafe: false }, { unpack: false }, { deob: false }, { human: true }, { fuel: DEFAULTS.fuel }, { rename: 'unique' }]);
  });
});

describe('moduleCommands', () => {
  it('lists the bundle, then the entry, then the modules by path, and marks the shown one', () => {
    const picked: Array<number | null> = [];
    const bundle = {
      kind: 'webpack',
      modules: [
        { id: '1', path: 'src/b.js', isEntry: false },
        { id: '0', path: 'src/index.js', isEntry: true },
        { id: '2', path: 'src/a.js', isEntry: false },
      ],
    };
    const list = moduleCommands(bundle, 2, (index) => picked.push(index));
    expect(list.map((item) => `${item.title}|${item.hint}`)).toEqual(['bundle|', 'src/index.js|entry', 'src/a.js|shown', 'src/b.js|']);
    list[0]?.run();
    list[3]?.run();
    expect(picked).toEqual([null, 0]);
  });

  it('is empty without a bundle', () => {
    expect(moduleCommands(null, null, () => undefined)).toEqual([]);
  });
});

describe('exampleCommands', () => {
  it('names each example and hands it to the picker', () => {
    const picked: string[] = [];
    const list = exampleCommands([{ name: 'webpack 5 bundle', source: 'fixtures/x/webpack-5.js', licence: null, bytes: 10 }], (example) => picked.push(example.source));
    expect(list.map((item) => `${item.group}|${item.title}|${item.hint}`)).toEqual(['Example|webpack 5 bundle|webpack-5.js']);
    list[0]?.run();
    expect(picked).toEqual(['fixtures/x/webpack-5.js']);
  });
});
