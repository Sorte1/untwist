import { CLI_DEFAULTS, type Settings } from './settings';

export function flagsFrom(settings: Settings): string {
  const words: string[] = [];
  const add = (on: boolean, ...flag: string[]) => {
    if (on) {
      words.push(...flag);
    }
  };
  add(!settings.unpack, '--no-unpack');
  add(!settings.jsx, '--no-jsx');
  add(settings.unsafe, '--unsafe');
  add(settings.rename !== 'none', '--rename', settings.rename);
  add(settings.fuel !== CLI_DEFAULTS.fuel, '--fuel', settings.fuel === null ? 'unlimited' : String(settings.fuel));
  add(!settings.deob, '--no-deobfuscate');
  add(!settings.unmin, '--no-unminify');
  add(settings.human, '--keep-human-names');
  add(settings.iter !== null, '--max-iterations', String(settings.iter));
  add(settings.single, '--single-file');
  add(settings.off !== '', '--disable', settings.off);
  add(settings.wasm !== 'off', '--wasm-check', settings.wasm);
  return words.join(' ');
}
