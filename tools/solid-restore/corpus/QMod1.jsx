const table = {};
table.k = "v";
const list = [];
for (let i = 0; i < 3; i++) list.push(i * 2);
const cfg = { a: 1 };
Object.assign(cfg, { b: 2 });
Object.freeze(cfg);
export default function QMod1() {
  return <p>{table.k}|{list.join()}|{cfg.b}|{Object.isFrozen(cfg) ? "frozen" : "open"}</p>;
}
