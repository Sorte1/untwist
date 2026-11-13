import { CLI_DEFAULTS, LIBRARY_PASSES, type Settings } from './settings';

function disabled(settings: Settings): string {
  const named = settings.off.split(',').filter((name) => name !== '');
  const extra = settings.libs ? [] : LIBRARY_PASSES.filter((name) => !named.includes(name));
  return [...named, ...extra].join(',');
}

function quoted(text: string): string {
  return /\s/.test(text) ? `"${text}"` : text;
}

export function flagsFrom(settings: Settings): string {
  const words: string[] = [];
  const add = (on: boolean, ...flag: string[]) => {
    if (on) {
      words.push(...flag);
    }
  };
  add(!settings.unpack, '--no-unpack');
  add(settings.split, '--split-hoisted');
  add(settings.split && settings.assume, '--split-assume-modules');
  add(!settings.jsx, '--no-jsx');
  add(settings.unsafe, '--unsafe');
  add(settings.rename !== 'none', '--rename', settings.rename);
  add(settings.fuel !== CLI_DEFAULTS.fuel, '--fuel', settings.fuel === null ? 'unlimited' : String(settings.fuel));
  add(!settings.deob, '--no-deobfuscate');
  add(!settings.unmin, '--no-unminify');
  add(settings.human, '--keep-human-names');
  add(settings.iter !== null, '--max-iterations', String(settings.iter));
  add(settings.single, '--single-file');
  add(settings.modern.trim() !== '', '--modern', quoted(settings.modern.trim()));
  add(settings.smap, '--source-map=inline');
  add(settings.smap && settings.imap, '--input-source-map=inline');
  add(settings.mem !== null, '--max-memory', `${settings.mem}MiB`);
  add(settings.time !== null, '--timeout', String(settings.time));
  const off = disabled(settings);
  add(off !== '', '--disable', off);
  add(settings.wasm !== 'off', '--wasm-check', settings.wasm);
  return words.join(' ');
}
