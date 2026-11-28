var out = [];
function emit(s) { out.push(String(s)); }
!function (root) {
  var registry = {};
  function define(name, value) {
    if (typeof registry[name] !== "undefined") {
      emit("dup " + name);
      return false;
    }
    registry[name] = value;
    return true;
  }
  function lookup(name) {
    var hit = registry[name];
    if (hit === undefined) {
      emit("miss " + name);
      return null;
    }
    return hit;
  }
  define("a", 1);
  define("a", 2);
  define("b", { deep: [1, 2, 3] });
  emit(lookup("a"));
  emit(lookup("zz"));
  var b = lookup("b");
  var len = b.deep.length;
  emit(len);
  root.api = { define: define, lookup: lookup };
}(this || {});
function parse(text) {
  var parts = text.split(",");
  var result = [];
  var i;
  for (i = 0; i < parts.length; i++) {
    var p = parts[i].trim();
    if (!p) continue;
    var n = Number(p);
    if (isNaN(n)) {
      result.push(p.toUpperCase());
    } else {
      result.push(n * 2);
    }
  }
  return result;
}
emit(parse("1, x, ,3").join("|"));
function walk(node, depth) {
  var label;
  label = node.name;
  if (depth > 0) {
    label = label + "@" + depth;
  }
  emit(label);
  var kids = node.children;
  if (kids) {
    for (var k = 0; k < kids.length; k++) walk(kids[k], depth + 1);
  }
}
walk({ name: "r", children: [{ name: "c1" }, { name: "c2", children: [{ name: "g" }] }] }, 0);
function retry(fn, times) {
  var last;
  while (times-- > 0) {
    try {
      return fn(times);
    } catch (e) {
      last = e;
    }
  }
  throw last;
}
emit(retry(function (t) { if (t > 1) throw new Error("t" + t); return "ok" + t; }, 4));
function sw(v) {
  switch (v) {
    case 1: return "one";
    case 2:
    case 3: return "few";
    default: return "many";
  }
}
emit([1, 2, 3, 9].map(sw).join());
var counter = 0;
function inc() { counter += 1; return counter; }
var r1 = inc();
var r2 = inc();
emit(r1 + r2);
var obj = { k: 1 };
if (obj.k) { obj.k = obj.k + 1; emit(obj.k); }
var flag = !obj.missing;
emit(flag);
var u = void 0;
emit(typeof u);
emit(1 / 0);
console.log(out.join("\n"));
