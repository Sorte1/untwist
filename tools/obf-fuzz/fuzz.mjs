import { readFileSync } from "node:fs";
import { join } from "node:path";

import { Rng } from "./rng.mjs";
import { featuresOf, program, sourceOf } from "./grammar.mjs";
import { sampleOptions } from "./options.mjs";

const FORMS = [
  ["script", 0.5],
  ["commonjs", 0.2],
  ["esm", 0.15],
  ["webpack5", 0.1],
  ["webpack4", 0.05],
];

function usage(message) {
  console.error(`obf-fuzz: ${message}
usage: node tools/obf-fuzz/fuzz.mjs programs --seed S --count N
       node tools/obf-fuzz/fuzz.mjs generate --seed S --start I --count N --out DIR
       node tools/obf-fuzz/fuzz.mjs rebuild --spec FILE --out DIR`);
  process.exit(2);
}

function args(argv) {
  const out = { command: argv[0] };
  for (let i = 1; i < argv.length; i += 2) {
    if (!argv[i].startsWith("--") || argv[i + 1] === undefined) usage(`bad argument ${argv[i]}`);
    out[argv[i].slice(2)] = argv[i + 1];
  }
  return out;
}

function pickForm(r) {
  let roll = r.next();
  for (const [form, weight] of FORMS) {
    roll -= weight;
    if (roll < 0) return form;
  }
  return FORMS[0][0];
}

export function draw(seed, index) {
  const r = new Rng(`${seed}/${index}`);
  const form = pickForm(r);
  const chunks = program(r, { form });
  const sampled = sampleOptions(r);
  const plainForm = form === "script" || form === "commonjs";
  return {
    id: `s${seed}-${index}`,
    seed: String(seed),
    index,
    form,
    chunks,
    obfuscator: sampled.obfuscator,
    options: sampled.options,
    obfSeed: sampled.seed,
    terser: r.chance(0.15),
    bundleMode: r.chance(0.5) ? "production" : "none",
    envelope: { strict: plainForm && r.chance(0.25), iife: plainForm && r.chance(0.2) },
  };
}

function number(text, name) {
  const value = Number(text);
  if (!Number.isInteger(value) || value < 0) usage(`--${name} wants a whole number`);
  return value;
}

async function main() {
  const a = args(process.argv.slice(2));
  if (a.command === "programs") {
    const count = number(a.count ?? "1", "count");
    for (let index = 0; index < count; index++) {
      const spec = draw(a.seed ?? "0", index);
      process.stdout.write(`${JSON.stringify({ id: spec.id, form: spec.form, features: featuresOf(spec.chunks), options: spec.options, source: sourceOf(spec.chunks) })}\n`);
    }
    return;
  }
  const { build } = await import("./build.mjs");
  if (a.command === "generate") {
    if (!a.out) usage("generate wants --out");
    const start = number(a.start ?? "0", "start");
    const count = number(a.count ?? "1", "count");
    for (let index = start; index < start + count; index++) {
      const spec = draw(a.seed ?? "0", index);
      const built = await build(spec, join(a.out, spec.id));
      process.stdout.write(`${JSON.stringify({ id: built.id, status: built.status })}\n`);
    }
    return;
  }
  if (a.command === "rebuild") {
    if (!a.spec || !a.out) usage("rebuild wants --spec and --out");
    const spec = JSON.parse(readFileSync(a.spec, "utf8"));
    const built = await build(spec, a.out);
    process.stdout.write(`${JSON.stringify({ id: built.id, status: built.status })}\n`);
    return;
  }
  usage(`unknown command ${a.command}`);
}

await main();
