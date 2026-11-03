import { closeBrackets, closeBracketsKeymap } from '@codemirror/autocomplete';
import type { Extension } from '@codemirror/state';
import { keymap } from '@codemirror/view';
import { formatter, type Formatter } from './format';
import { notices } from './notice';
import { shortcutSheet } from './shortcuts';

const formatters = new WeakMap<Promise<WebAssembly.Module>, Formatter>();

function formatterFor(module: Promise<WebAssembly.Module>): Formatter {
  const known = formatters.get(module);
  if (known !== undefined) {
    return known;
  }
  const made = formatter(module);
  formatters.set(module, made);
  return made;
}

export function editingExtras(module: Promise<WebAssembly.Module>): Extension {
  return [
    closeBrackets(),
    notices(),
    shortcutSheet(),
    keymap.of([...closeBracketsKeymap, { key: 'Shift-Alt-f', run: formatterFor(module), preventDefault: true }]),
  ];
}
