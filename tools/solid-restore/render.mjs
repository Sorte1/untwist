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
for (const key of ["Event", "CustomEvent", "EventTarget"]) {
  Object.defineProperty(globalThis, key, { value: window[key], configurable: true, writable: true });
}
globalThis.window = window;
globalThis.document = window.document;
Object.defineProperty(globalThis, "navigator", { value: window.navigator, configurable: true });

const logs = [];
console.log = (...args) => logs.push(args.map((arg) => String(arg)).join(" "));
console.warn = () => {};
console.error = () => {};

const { render, createComponent } = await import("solid-js/web");
const Component = (await import(pathToFileURL(componentPath).href)).default;
const target = document.createElement("main");
document.body.appendChild(target);

const settle = async () => {
  for (let round = 0; round < 3; round += 1) {
    await Promise.resolve();
    await new Promise((resolve) => setTimeout(resolve, 0));
  }
};
const html = () => target.innerHTML + "\n" + [...target.querySelectorAll("input, textarea, select")].map((field) => `${field.value}|${field.checked}`).join(",");
const pick = (selector) => {
  const found = target.querySelector(selector);
  if (!found) throw new Error(`no element matches ${selector}`);
  return found;
};

const snapshots = [];
const errors = [];
try {
  render(() => createComponent(Component, spec.props ?? {}), target);
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
process.stdout.write(JSON.stringify({ snapshots, logs, errors }));
process.exit(0);
