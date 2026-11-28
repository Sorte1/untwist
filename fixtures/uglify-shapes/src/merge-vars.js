var out = [];
function first(s) {
  var m = /(\d+)/.exec(s);
  if (m) {
    out.push("num " + m[1]);
  }
  var w = /([a-z]+)/.exec(s);
  if (w) out.push("word " + w[1]); else out.push("none");
}
first("ab12"); first("??");
function grow(store, key) {
  var list = store[key];
  if (!list) {
    list = store[key] = [];
  }
  list.push(key.length);
  return list.length;
}
var st = {};
out.push(grow(st, "aa"), grow(st, "aa"), grow(st, "b"));
function sum(arr) {
  var total = 0;
  var item;
  for (var i = 0; i < arr.length; i++) {
    item = arr[i];
    total += item;
  }
  return total;
}
out.push(sum([1, 2, 3]));
function label(v) {
  var kind = typeof v;
  var text = kind === "object" ? (v === null ? "null" : "obj") : kind;
  out.push(text);
  return text;
}
label(null); label({}); label(3);
var el = { style: {} };
function setUp(e) {
  var s = e.style;
  s.color = "red";
  s.width = "1px";
  out.push(s.color + s.width);
}
setUp(el);
console.log(out.join("\n"));
