const inventory = {
  apples: 12,
  pears: 7,
  plums: 30,
  nested: { depth: 2, label: "inner" },
};

const extra = { cherries: 99, ...inventory };
const { apples, pears, ...others } = extra;

function describe(item) {
  return item.label + " at depth " + item.depth;
}

const shape = {
  width: 6,
  height: 4,
  get area() {
    return this.width * this.height;
  },
  scale(factor) {
    return { width: this.width * factor, height: this.height * factor };
  },
};

const keepSettings = { keepAlive: true, retries: 3 };
const scaled = shape.scale(3);
console.log("fruit", apples + pears, Object.keys(others).length);
console.log("describe", describe(inventory.nested));
console.log("area", shape.area, scaled.width, scaled.height);
console.log("settings", keepSettings.keepAlive, keepSettings.retries);
console.log("json", JSON.stringify({ total: apples + pears + inventory.plums, ok: true }));
console.log("entries", Object.entries({ left: 1, right: 2 }).length);
