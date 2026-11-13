var name = "world";
var n = 3;
console.log(`hello ${name}`, `${n} + ${n} = ${n + n}`, `nested ${`inner ${name}`}`);
console.log(`multi
line`, `${{
  toString() {
    return "obj";
  }
}}`, `${[1, 2]}`);
function tag(strings) {
  for (var _len = arguments.length, values = new Array(_len > 1 ? _len - 1 : 0), _key = 1; _key < _len; _key++) {
    values[_key - 1] = arguments[_key];
  }
  return strings.raw.join("|") + "#" + strings.join("|") + "#" + values.join(",");
}
console.log(tag`a${1}b${2}c`, tag`\n${"x"}\t`);
function same(s) {
  return s;
}
var first = function first() {
  return same`x`;
};
console.log(first() === first(), Object.isFrozen(first()), String.raw`\u{41}${1}\n`);
var sym = {
  valueOf() {
    return 1;
  },
  toString() {
    return "str";
  }
};
console.log(`${sym}`, "" + sym);
