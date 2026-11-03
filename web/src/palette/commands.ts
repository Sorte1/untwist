import type { Example } from '../examples/examples';
import { RENAME_MODES, DEFAULTS, type Settings } from '../settings/settings';
import { treeOrder, type BundleInfo } from '../ui/module-tree';
import type { Command } from './match';

type Flag = 'unpack' | 'jsx' | 'deob' | 'unmin' | 'human' | 'single';

const SWITCHES: ReadonlyArray<readonly [Flag, string]> = [
  ['unpack', 'Unpack bundles'],
  ['jsx', 'Decompile JSX'],
  ['deob', 'Deobfuscate'],
  ['unmin', 'Unminify'],
  ['human', 'Keep human names'],
  ['single', 'Single file'],
];

const onOff = (on: boolean): string => (on ? 'on' : 'off');
const current = (on: boolean): string => (on ? 'current' : '');

function baseName(path: string): string {
  return path.slice(path.lastIndexOf('/') + 1);
}

export function settingCommands(settings: Settings, apply: (patch: Partial<Settings>) => void): Command[] {
  return [
    { group: 'Mode', title: 'Safe mode', hint: current(!settings.unsafe), run: () => apply({ unsafe: false }) },
    { group: 'Mode', title: 'Unsafe mode', hint: current(settings.unsafe), run: () => apply({ unsafe: true }) },
    ...SWITCHES.map(([key, title]) => ({ group: 'Setting', title, hint: onOff(settings[key]), run: () => apply({ [key]: !settings[key] }) })),
    {
      group: 'Setting',
      title: 'No fuel limit',
      hint: onOff(settings.fuel === null),
      run: () => apply({ fuel: settings.fuel === null ? DEFAULTS.fuel : null }),
    },
    ...RENAME_MODES.map((mode) => ({ group: 'Rename', title: `Rename: ${mode}`, hint: current(settings.rename === mode), run: () => apply({ rename: mode }) })),
  ];
}

export function moduleCommands(bundle: BundleInfo | null, shown: number | null, pick: (index: number | null) => void): Command[] {
  if (bundle === null) {
    return [];
  }
  const modules = treeOrder(bundle.modules).map((index) => {
    const module = bundle.modules[index];
    const hint = index === shown ? 'shown' : module?.isEntry === true ? 'entry' : '';
    return { group: 'Module', title: module?.path ?? `module ${index}`, hint, run: () => pick(index) };
  });
  return [{ group: 'Module', title: 'bundle', hint: shown === null ? 'shown' : '', run: () => pick(null) }, ...modules];
}

export function exampleCommands(examples: readonly Example[], pick: (example: Example) => void): Command[] {
  return examples.map((example) => ({ group: 'Example', title: example.name, hint: baseName(example.source), run: () => pick(example) }));
}
