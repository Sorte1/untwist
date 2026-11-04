const pick = (a, b) => (a > 1 ? "x" + b : "y" + b + "!");
console.log(pick(2, "a"), pick(0, "b"));
