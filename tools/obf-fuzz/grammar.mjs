export const HAZARDS = ["observes-global-names", "patches-builtin", "text-from-data", "top-fn-prints-param"];

const q = (text) => JSON.stringify(text);

const SNIPPETS = {
  strings(r, n) {
    const words = Array.from({ length: r.range(2, 5) }, () => r.word());
    return {
      code: `const words_${n} = [${words.map(q).join(", ")}];\nconsole.log("strings ${n}", words_${n}.join("-"), words_${n}.map((w) => w.toUpperCase()).join(""), ${q(r.word())} + ${q(r.word())});`,
    };
  },
  "object-spread"(r, n) {
    return {
      code: `const base_${n} = { ${r.word()}: ${r.int(90)}, label: ${q(r.word())} };\nconst more_${n} = { extra: ${q(r.word())}, ...base_${n} };\nconsole.log("spread ${n}", JSON.stringify(more_${n}), Object.keys(more_${n}).length);`,
    };
  },
  "object-rest"(r, n) {
    return {
      code: `const { first: first_${n}, ...rest_${n} } = { first: ${q(r.word())}, second: ${r.int(50)}, third: ${q(r.word())} };\nconsole.log("rest ${n}", first_${n}, Object.keys(rest_${n}).join(","));`,
    };
  },
  getter(r, n) {
    return {
      code: `const box_${n} = {\n  width: ${r.range(1, 9)},\n  height: ${r.range(1, 9)},\n  get area() {\n    return this.width * this.height;\n  },\n  set area(v) {\n    this.width = v;\n    this.height = 1;\n  },\n};\nconsole.log("getter ${n}", box_${n}.area);\nbox_${n}.area = ${r.range(10, 40)};\nconsole.log("setter ${n}", box_${n}.width, box_${n}.area, ${q(r.word())});`,
    };
  },
  classes(r, n) {
    return {
      code: `class Base_${n} {\n  constructor(name) {\n    this.name = name;\n  }\n  describe() {\n    return this.name + ${q(" is " + r.word())};\n  }\n  static make(name) {\n    return new this(name);\n  }\n}\nclass Child_${n} extends Base_${n} {\n  describe() {\n    return super.describe() + ${q(" and " + r.word())};\n  }\n}\nconsole.log("class ${n}", Child_${n}.make(${q(r.word())}).describe(), typeof Base_${n}.make);`,
    };
  },
  "class-fields"(r, n) {
    return {
      code: `class Counter_${n} {\n  count = ${r.int(5)};\n  static label = ${q(r.word())};\n  #secret = ${q(r.word())};\n  bump() {\n    this.count += 1;\n    return this;\n  }\n  get secret() {\n    return this.#secret;\n  }\n}\nconst counter_${n} = new Counter_${n}().bump().bump();\nconsole.log("fields ${n}", counter_${n}.count, Counter_${n}.label, counter_${n}.secret);`,
      features: ["modern-syntax"],
    };
  },
  "map-set"(r, n) {
    const keys = Array.from({ length: r.range(2, 4) }, () => r.word());
    return {
      code: `const map_${n} = new Map();\n${keys.map((k, i) => `map_${n}.set(${q(k)}, ${i});`).join("\n")}\nconst set_${n} = new Set([${keys.map(q).join(", ")}, ${q(keys[0])}]);\nconst weak_${n} = new WeakMap([[map_${n}, ${q(r.word())}]]);\nconsole.log("map ${n}", map_${n}.size, [...map_${n}.keys()].join("|"), map_${n}.get(${q(keys[0])}), set_${n}.size, weak_${n}.get(map_${n}));`,
    };
  },
  "symbol-key"(r, n) {
    return {
      code: `const sym_${n} = Symbol(${q(r.word())});\nconst holder_${n} = { [sym_${n}]: ${q(r.word())}, visible: ${q(r.word())} };\nconsole.log("symbol ${n}", holder_${n}[sym_${n}], Object.keys(holder_${n}).length, String(sym_${n}.description).length);`,
    };
  },
  async(r, n) {
    return {
      code: `async function later_${n}(label, value) {\n  await null;\n  return label + "=" + value;\n}\nlater_${n}(${q(r.word())}, ${r.int(100)}).then((text) => console.log("async ${n}", text));\nPromise.reject(new Error(${q(r.word())})).catch((error) => console.log("caught ${n}", error.message));`,
    };
  },
  generator(r, n) {
    return {
      code: `function* steps_${n}(limit) {\n  for (let i = 0; i < limit; i += ${r.range(1, 3)}) {\n    yield ${q(r.word())} + i;\n  }\n}\nconsole.log("generator ${n}", [...steps_${n}(${r.range(3, 9)})].join(" "));`,
    };
  },
  require(r, n, ctx) {
    if (ctx.form === "esm") {
      return {
        code: `import * as path_${n} from "node:path";\nconsole.log("path ${n}", path_${n}.basename(${q(`/srv/${r.word()}/${r.word()}.txt`)}, ".txt"), path_${n}.extname(${q(r.word() + ".tar.gz")}));`,
        features: ["import"],
      };
    }
    const id = ctx.form === "webpack4" ? "path" : "node:path";
    return {
      code: `const path_${n} = require(${q(id)});\nconsole.log("path ${n}", path_${n}.basename(${q(`/srv/${r.word()}/${r.word()}.txt`)}, ".txt"), path_${n}.extname(${q(r.word() + ".tar.gz")}));`,
    };
  },
  closure(r, n) {
    return {
      code: `function counter_${n}(start) {\n  let value = start;\n  return { next: () => ++value, name: ${q(r.word())} };\n}\nconst tick_${n} = counter_${n}(${r.int(20)});\ntick_${n}.next();\nconsole.log("closure ${n}", tick_${n}.next(), tick_${n}.name);`,
    };
  },
  "try-finally"(r, n) {
    return {
      code: `function guarded_${n}(flag) {\n  const log = [];\n  try {\n    if (flag) throw new TypeError(${q(r.word())});\n    log.push(${q(r.word())});\n    return log;\n  } catch (error) {\n    log.push(error.name, error.message);\n    return log;\n  } finally {\n    log.push(${q(r.word())});\n  }\n}\nconsole.log("try ${n}", guarded_${n}(true).join(","), guarded_${n}(false).join(","));`,
    };
  },
  "labelled-loop"(r, n) {
    return {
      code: `const found_${n} = [];\nouter_${n}: for (let i = 0; i < ${r.range(3, 6)}; i++) {\n  for (let j = 0; j < 4; j++) {\n    if (j > i) continue outer_${n};\n    if (i * j > ${r.range(4, 9)}) break outer_${n};\n    found_${n}.push(${q(r.word().slice(0, 2))} + i + j);\n  }\n}\nconsole.log("labels ${n}", found_${n}.join(" "));`,
    };
  },
  regex(r, n) {
    return {
      code: `const stamp_${n} = ${q(`${r.range(2000, 2030)}-${r.range(10, 12)}-${r.range(10, 28)}`)};\nconsole.log("regex ${n}", stamp_${n}.match(/(\\d+)-(\\d+)-(\\d+)/).slice(1).join("/"), ${q(r.word() + " " + r.word())}.replace(/\\s+/g, "_"), /^[a-z]+$/i.test(${q(r.word())}));`,
    };
  },
  template(r, n) {
    return {
      code: `function tag_${n}(parts, ...values) {\n  return parts.raw.join("|") + "#" + values.join(",");\n}\nconst who_${n} = ${q(r.word())};\nconsole.log("template ${n}", \`hello \${who_${n}} \${${r.int(9)} + 1}\`, tag_${n}\`a\${who_${n}}b\${${r.int(9)}}c\`);`,
    };
  },
  switch(r, n) {
    const words = Array.from({ length: 3 }, () => r.word());
    return {
      code: `function pick_${n}(k) {\n  switch (k) {\n    case ${q(words[0])}:\n      return 1;\n    case ${q(words[1])}:\n    case ${q(words[2])}:\n      return 2;\n    default:\n      return k.length;\n  }\n}\nconsole.log("switch ${n}", ${words.map((w) => `pick_${n}(${q(w)})`).join(", ")}, pick_${n}(${q(r.word())}));`,
    };
  },
  destructuring(r, n) {
    return {
      code: `const [head_${n}, , third_${n} = ${q(r.word())}, ...tail_${n}] = [${q(r.word())}, ${q(r.word())}, undefined, ${r.int(9)}, ${r.int(9)}];\nconst swap_${n} = ([a, b]) => [b, a];\nconsole.log("destructure ${n}", head_${n}, third_${n}, tail_${n}.length, swap_${n}([${q(r.word())}, 1]).join(":"));`,
    };
  },
  "optional-chaining"(r, n) {
    return {
      code: `const deep_${n} = { inner: { label: ${q(r.word())} }, list: null };\nconsole.log("optional ${n}", deep_${n}.inner?.label, deep_${n}.list?.length ?? ${q(r.word())}, deep_${n}.missing?.x);`,
      features: ["modern-syntax"],
    };
  },
  "computed-member"(r, n) {
    const key = r.word();
    return {
      code: `const table_${n} = { ${key}: ${q(r.word())}, ["dyn" + ${r.int(9)}]: ${r.int(99)} };\nconst which_${n} = ${q(key)};\nconsole.log("computed ${n}", table_${n}[which_${n}], Object.entries(table_${n}).map(([k, v]) => k + v).sort().join(";"));`,
    };
  },
  json(r, n) {
    return {
      code: `const record_${n} = JSON.parse(${q(JSON.stringify({ id: r.int(1000), tag: r.word(), list: [r.word(), r.int(9)] }))});\nconsole.log("json ${n}", record_${n}.tag, JSON.stringify(record_${n}.list), JSON.stringify({ ok: true, n: record_${n}.id }));`,
    };
  },
  "nonstrict-this"(r, n) {
    return {
      code: `function whoAmI_${n}() {\n  return typeof this;\n}\nconsole.log("this ${n}", whoAmI_${n}(), ${q(r.word())});`,
    };
  },
  "module-exports"(r, n, ctx) {
    if (ctx.form === "esm") {
      return {
        code: `export const exported_${n} = ${q(r.word())};\nconsole.log("export ${n}", exported_${n}.length);`,
        features: ["export"],
      };
    }
    return {
      code: `module.exports.exported_${n} = ${q(r.word())};\nconsole.log("export ${n}", typeof module.exports.exported_${n});`,
    };
  },
  "observes-global-names"(r, n) {
    return {
      code: `let globals_${n} = 0;\nfor (const key in globalThis) {\n  if (key.startsWith("_0x")) globals_${n}++;\n}\nconsole.log("globals ${n}", globals_${n}, Object.keys(globalThis).filter((k) => k.startsWith("_0x")).length, ${q(r.word())});`,
    };
  },
  "patches-builtin"(r, n) {
    return {
      code: `const patchKey_${n} = process.pid > 0 ? ${q("fuzz" + r.word())} : "other";\nNumber.prototype[patchKey_${n}] = function () {\n  return this.valueOf() + 1;\n};\nconsole.log("patched ${n}", (${r.int(9)})[patchKey_${n}](), ${q(r.word())});`,
    };
  },
  "text-from-data"(r, n) {
    return {
      code: `const source_${n} = globalThis.fuzzSource_${n} !== undefined ? globalThis.fuzzSource_${n} : ${q(`${r.int(9)} * ${r.int(9)}`)};\nconsole.log("eval ${n}", eval(source_${n}), ${q(r.word())});`,
    };
  },
  "top-fn-prints-param"(r, n) {
    return {
      code: `function show_${n}(value) {\n  console.log("show ${n}", typeof value, JSON.stringify(value));\n}\nshow_${n}({ ${r.word()}: ${q(r.word())} });`,
    };
  },
};

export const FEATURES = Object.keys(SNIPPETS);
const PLAIN = FEATURES.filter((f) => !HAZARDS.includes(f));
const HAZARD_RATE = 0.15;

export function snippet(name, r, n, ctx) {
  const made = SNIPPETS[name](r, n, ctx);
  return { code: made.code, features: [name, ...(made.features ?? [])] };
}

const MODERN = ["class-fields", "optional-chaining"];
const MODULE_ONLY = ["require", "module-exports"];

export function allowed(form) {
  if (form === "script") return PLAIN.filter((f) => !MODULE_ONLY.includes(f));
  if (form === "webpack4") return PLAIN.filter((f) => !MODERN.includes(f));
  return PLAIN;
}

export function program(r, ctx) {
  const count = r.range(2, 7);
  const plain = allowed(ctx.form);
  const chunks = [];
  for (let n = 0; n < count; n++) {
    const name = r.chance(HAZARD_RATE) ? r.pick(HAZARDS) : r.pick(plain);
    chunks.push(snippet(name, r, n, ctx));
  }
  return chunks;
}

export function featuresOf(chunks) {
  return [...new Set(chunks.flatMap((c) => c.features))].sort();
}

export function sourceOf(chunks) {
  return chunks.map((c) => c.code).join("\n\n") + "\n";
}
