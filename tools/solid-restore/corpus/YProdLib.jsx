export function produce(base, recipe) { const draft = { ...base }; recipe(draft); console.log("my-produce"); return draft; }
export function batch(list, size) { const out = []; for (let i = 0; i < list.length; i += size) out.push(list.slice(i, i + size)); return out; }
export function unwrap(x) { return x && x.value; }
export default function YProdLib() { return null; }
