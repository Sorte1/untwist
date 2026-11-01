'use strict';
const roots = [
  'Object', 'Function', 'Array', 'Number', 'parseFloat', 'parseInt', 'Boolean', 'String', 'Symbol', 'Date',
  'Promise', 'RegExp', 'Error', 'AggregateError', 'EvalError', 'RangeError', 'ReferenceError', 'SyntaxError',
  'TypeError', 'URIError', 'JSON', 'Math', 'Intl', 'ArrayBuffer', 'Atomics', 'Uint8Array', 'Int8Array',
  'Uint16Array', 'Int16Array', 'Uint32Array', 'Int32Array', 'Float32Array', 'Float64Array', 'Uint8ClampedArray',
  'BigUint64Array', 'BigInt64Array', 'DataView', 'Map', 'BigInt', 'Set', 'WeakMap', 'WeakSet', 'Proxy', 'Reflect',
  'FinalizationRegistry', 'WeakRef', 'decodeURI', 'decodeURIComponent', 'encodeURI', 'encodeURIComponent',
  'escape', 'unescape', 'eval', 'isFinite', 'isNaN', 'SharedArrayBuffer', 'Iterator',
];
const segmenter = typeof Intl.Segmenter === 'function' ? new Intl.Segmenter().segment('') : undefined;
const hidden = {
  '%TypedArray%': Object.getPrototypeOf(Uint8Array),
  '%GeneratorFunction%': Object.getPrototypeOf(function* () {}).constructor,
  '%AsyncFunction%': Object.getPrototypeOf(async function () {}).constructor,
  '%AsyncGeneratorFunction%': Object.getPrototypeOf(async function* () {}).constructor,
  '%ArrayIteratorPrototype%': Object.getPrototypeOf([][Symbol.iterator]()),
  '%MapIteratorPrototype%': Object.getPrototypeOf(new Map()[Symbol.iterator]()),
  '%SetIteratorPrototype%': Object.getPrototypeOf(new Set()[Symbol.iterator]()),
  '%StringIteratorPrototype%': Object.getPrototypeOf(''[Symbol.iterator]()),
  '%RegExpStringIteratorPrototype%': Object.getPrototypeOf(/a/[Symbol.matchAll]('')),
  '%AsyncIteratorPrototype%': Object.getPrototypeOf(Object.getPrototypeOf(Object.getPrototypeOf((async function* () {})()))),
  '%ThrowTypeError%': Object.getOwnPropertyDescriptor(Function.prototype, 'caller').get,
  '%WrapForValidIteratorPrototype%': Object.getPrototypeOf(Iterator.from({ next() { return { done: true }; } })),
  '%IteratorHelperPrototype%': Object.getPrototypeOf([].values().map((x) => x)),
  '%SegmentsPrototype%': segmenter && Object.getPrototypeOf(segmenter),
  '%SegmentIteratorPrototype%': segmenter && Object.getPrototypeOf(segmenter[Symbol.iterator]()),
};
const keyName = (k) => (typeof k === 'symbol' ? '@@' + String(k.description).replace(/^Symbol\./, '') : String(k));
const isObject = (v) => v !== null && (typeof v === 'object' || typeof v === 'function');
const seen = new Map();
const keysOf = new Map();
const found = [];
const note = (fn, key) => {
  if (!keysOf.has(fn)) keysOf.set(fn, new Set());
  keysOf.get(fn).add(key);
};
let queue = [];
let later = [];
const enqueue = (value, path, list) => {
  if (isObject(value) && !seen.has(value)) {
    seen.set(value, path);
    list.push([value, path]);
  }
};
for (const name of roots) {
  if (name in globalThis) {
    note(globalThis[name], name);
    enqueue(globalThis[name], name, queue);
  }
}
for (const [name, value] of Object.entries(hidden)) {
  if (value !== undefined) enqueue(value, name, queue);
}
while (queue.length || later.length) {
  if (!queue.length) {
    queue = [];
    for (const [value, path] of later) enqueue(value, path, queue);
    later = [];
    if (!queue.length) break;
  }
  const next = [];
  for (const [value, path] of queue) {
    if (typeof value === 'function') found.push([path, value]);
    for (const key of Reflect.ownKeys(value)) {
      const d = Object.getOwnPropertyDescriptor(value, key);
      const sub = path + '.' + keyName(key);
      if ('value' in d) {
        if (typeof d.value === 'function') note(d.value, keyName(key));
        enqueue(d.value, sub, next);
      }
      if (d.get) {
        note(d.get, keyName(key) + ' [get]');
        enqueue(d.get, sub + ' [get]', next);
      }
      if (d.set) {
        note(d.set, keyName(key) + ' [set]');
        enqueue(d.set, sub + ' [set]', next);
      }
    }
    const proto = Object.getPrototypeOf(value);
    if (isObject(proto) && !seen.has(proto)) later.push([proto, path + '.[[Prototype]]']);
  }
  queue = next;
}
const rows = found.map(([path, fn]) => path + '\t' + [...(keysOf.get(fn) || [])].sort().join(','));
for (const row of rows.sort()) console.log(row);
