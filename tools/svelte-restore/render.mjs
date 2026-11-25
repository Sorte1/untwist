import { JSDOM } from "jsdom";
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

const [componentPath, specPath] = process.argv.slice(2);
const spec = specPath ? JSON.parse(readFileSync(specPath, "utf8")) : {};
const dom = new JSDOM("<!doctype html><html><head></head><body></body></html>", { pretendToBeVisual: true });
const { window } = dom;
for (const key of Object.getOwnPropertyNames(window)) {
  if (!(key in globalThis)) {
    try {
      globalThis[key] = window[key];
    } catch {}
  }
}
globalThis.window = window;
globalThis.document = window.document;
Object.defineProperty(globalThis, "navigator", { value: window.navigator, configurable: true });

const logs = [];
console.log = (...args) => logs.push(args.map((arg) => String(arg)).join(" "));
console.warn = () => {};

const { mount, flushSync, tick } = await import("svelte");
const Component = (await import(pathToFileURL(componentPath).href)).default;
const target = document.createElement("main");
document.body.appendChild(target);

const settle = async () => {
  flushSync();
  await tick();
  await new Promise((resolve) => setTimeout(resolve, 0));
  flushSync();
};
const html = () => target.innerHTML.replace(/svelte-[a-z0-9]+/g, "svelte-H");
const css = () => [...document.head.querySelectorAll("style")].map((style) => style.textContent.replace(/svelte-[a-z0-9]+/g, "svelte-H")).join("\n");
const pick = (selector) => {
  const found = target.querySelector(selector);
  if (!found) throw new Error(`no element matches ${selector}`);
  return found;
};

const snapshots = [];
const errors = [];
try {
  mount(Component, { target, props: spec.props ?? {} });
  await settle();
  snapshots.push(html());
  for (const step of spec.steps ?? []) {
    if (step.click) pick(step.click).click();
    if (step.check) pick(step.check).click();
    if (step.input) {
      const element = pick(step.input[0]);
      element.value = step.input[1];
      element.dispatchEvent(new window.Event("input", { bubbles: true }));
    }
    if (step.event) pick(step.event[0]).dispatchEvent(new window.Event(step.event[1], { bubbles: true }));
    if (step.wait) await new Promise((resolve) => setTimeout(resolve, step.wait));
    await settle();
    snapshots.push(html());
  }
} catch (error) {
  errors.push(String(error && error.message ? error.message : error));
}
process.stdout.write(JSON.stringify({ snapshots, logs, css: css(), errors }));
process.exit(0);
