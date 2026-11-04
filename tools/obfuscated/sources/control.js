function classify(value) {
  switch (typeof value) {
    case "number":
      if (value < 0) {
        return "negative";
      }
      return value % 2 === 0 ? "even" : "odd";
    case "string":
      return "text of " + value.length;
    case "boolean":
    case "undefined":
      return "flag";
    default:
      return "other";
  }
}

function parse(text) {
  try {
    if (text.length === 0) {
      throw new Error("empty input");
    }
    return JSON.parse(text);
  } catch (error) {
    return "failed: " + error.message;
  } finally {
    console.log("parsed", text.length);
  }
}

let total = 0;
for (let i = 0; i < 10; i++) {
  if (i === 3) {
    continue;
  }
  if (i === 8) {
    break;
  }
  total += i;
}

let countdown = 5;
const seen = [];
while (countdown > 0) {
  seen.push(countdown);
  countdown -= 2;
}

let rounds = 0;
do {
  rounds++;
} while (rounds < 4);

outer: for (let row = 0; row < 3; row++) {
  for (let col = 0; col < 3; col++) {
    if (col > row) {
      continue outer;
    }
    if (row === 2 && col === 1) {
      break outer;
    }
    total += row * 10 + col;
  }
}

const record = { alpha: 1, beta: 2 };
const keys = [];
for (const key in record) {
  keys.push(key);
}

console.log("classify", [7, -1, 4, "abc", true, undefined, null].map(classify).join(","));
console.log("parse", parse("[1,2]").length, parse(""));
console.log("loops", total, seen.join("-"), rounds, keys.join("+"));
