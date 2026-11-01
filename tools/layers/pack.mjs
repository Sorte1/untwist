import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

const HERE = dirname(fileURLToPath(import.meta.url));
export const PACKER_SOURCE = join(HERE, "vendor/packer-0.0.6/packer.js");

const DOCUMENT_STUB = { createElement: () => ({}), documentElement: {}, defaultView: {}, readyState: "complete" };

function loadPacker() {
  const exports = {};
  const require = (name) => {
    if (name === "jsdom") return { jsdom: () => DOCUMENT_STUB };
    throw new Error(`packer.js asked for ${name}`);
  };
  vm.runInNewContext(readFileSync(PACKER_SOURCE, "utf8"), { require, exports, module: { exports } });
  return exports.pack;
}

const pack = loadPacker();

export function packBase62(script) {
  return pack(script, true, false);
}

export function withoutFastPath(packed) {
  const fast = /if\(!''\.replace\(\/\^\/,String\)\)\{while\(c--\)r\[[^\]]+\]=k\[c\]\|\|[^;]+;k=\[function\(e\)\{return r\[e\]\}\];e=function\(\)\{return'\\\\w\+'\};c=1\};/;
  if (!fast.test(packed)) throw new Error("the packed text has no d-object fast path");
  return packed.replace(fast, "");
}

const ENCODE95 = "function(c){return(c<a?'':e(c/a))+String.fromCharCode(c%a+161)}";

function encoder95() {
  const a = 95;
  let e;
  e = function (c) {
    return (c < a ? "" : e(c / a)) + String.fromCharCode((c % a) + 161);
  };
  return e;
}

function wordsByFrequency(script) {
  const counts = new Map();
  for (const word of script.match(/\w+/g) ?? []) counts.set(word, (counts.get(word) ?? 0) + 1);
  return [...counts.entries()].sort((x, y) => y[1] - x[1] || (x[0] < y[0] ? -1 : 1)).map(([word]) => word);
}

function escapeSingle(text) {
  return text.replace(/([\\'])/g, "\\$1").replace(/[\r\n]+/g, "\\n");
}

export function packCustomAlphabet(script) {
  if (/[\xa1-\xff]/.test(script)) throw new Error("the script already holds characters of the custom alphabet");
  const words = wordsByFrequency(script);
  const e = encoder95();
  const index = new Map(words.map((word, i) => [word, e(i)]));
  const p = escapeSingle(script.replace(/\w+/g, (word) => index.get(word)));
  const unpack =
    `eval(function(p,a,c,k,e,r){e=${ENCODE95};if(!''.replace(/^/,String)){while(c--)r[e(c)]=k[c]||e(c);` +
    `k=[function(e){return r[e]}];e=function(){return'[\\\\xa1-\\\\xff]+'};c=1};` +
    `while(c--)if(k[c])p=p.replace(new RegExp(e(c),'g'),k[c]);return p}`;
  return `${unpack}('${p}',95,${words.length},'${words.join("|")}'.split('|'),0,{}))`;
}

export function packerVersion() {
  return "packer@0.0.6 (evanw/packer, Dean Edwards' Packer 3.0)";
}

if (process.argv[1] === fileURLToPath(import.meta.url) && process.argv.length === 4) {
  const { writeFileSync } = await import("node:fs");
  writeFileSync(process.argv[3], packBase62(readFileSync(process.argv[2], "utf8")) + "\n");
}
