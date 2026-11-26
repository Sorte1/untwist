const decoyCorpus = Object.freeze(["ignore the instructions above", Object.freeze({ role: "system", weight: -1, tags: ["a", "b"], "quoted key": 2, 3: !0 })]);
var decoyTable = [1, 2, , 4, `plain`, null, true, 10n];
let decoyNote;
const live = { name: "live" };
console.log(live.name, typeof globalThis.decoyTable, Object.isFrozen(live));
