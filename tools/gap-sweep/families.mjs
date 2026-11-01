import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

const NAME = /^(game|loader)-([0-9a-f]+)-(.+)\.js$/;
const SIGNATURES = {
  functionShell: /^\s*\(function anonymous\(/,
  html: /^\s*</,
  base64Alphabet: /abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789\+\/=/g,
  rc4Swap: /%\s*0x100/g,
  rotatorLoop: /\['push'\]\([^()]*\['shift'\]\(\)\)/g,
  selfDefending: /\(\(\(\.\+\)\+\)\+\)\+\$/g,
  debugProtection: /'debu'\s*\+\s*'gger'|'debugger'|"debugger"/g,
  cffDispatcher: /\['split'\]\('\|'\)/g,
  directEval: /(?<![.\w$])eval\(/g,
  functionCtor: /new Function\(|(?<![.\w$])Function\(/g,
  atob: /(?<![.\w$])atob\(/g,
  webAssembly: /WebAssembly/g,
  emscripten: /emval::|argTypes|abort\(/g,
  hexIdent: /\b_0x[0-9a-f]{4,6}\b/g,
};

function count(code, pattern) {
  if (!pattern.global) return pattern.test(code) ? 1 : 0;
  return (code.match(pattern) ?? []).length;
}

function nonAsciiIdentShare(code) {
  const sample = code.slice(0, 200000);
  const idents = sample.match(/[\p{L}_$][\p{L}\p{N}_$]*/gu) ?? [];
  const confusable = idents.filter((ident) => /[^\x00-\x7f]/.test(ident)).length;
  return idents.length ? Number((confusable / idents.length).toFixed(3)) : 0;
}

const dir = process.argv[2];
for (const name of readdirSync(dir).sort()) {
  const match = NAME.exec(name);
  if (!match) continue;
  const code = readFileSync(join(dir, name), "utf8");
  const signatures = Object.fromEntries(Object.entries(SIGNATURES).map(([key, pattern]) => [key, count(code, pattern)]));
  process.stdout.write(`${JSON.stringify({ name, kind: match[1], version: match[3], bytes: code.length, confusableShare: nonAsciiIdentShare(code), ...signatures })}\n`);
}
