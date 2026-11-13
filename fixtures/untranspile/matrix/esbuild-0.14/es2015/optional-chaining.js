var _a, _b, _c, _d, _e, _f, _g, _h, _i, _j, _k, _l, _m, _n, _o, _p;
const data = { user: { name: "ann", tags: ["a"], greet() {
  return "hi " + this.name;
} }, empty: null };
let calls = 0;
const get = () => {
  calls++;
  return data;
};
console.log((_a = data.user) == null ? void 0 : _a.name, (_b = data.empty) == null ? void 0 : _b.name, (_c = data.missing) == null ? void 0 : _c.deep.deeper);
console.log((_e = (_d = data.user) == null ? void 0 : _d.tags) == null ? void 0 : _e[0], (_f = data.empty) == null ? void 0 : _f[0], (_h = (_g = data.user).greet) == null ? void 0 : _h.call(_g), (_j = (_i = data.user).nope) == null ? void 0 : _j.call(_i));
console.log((_l = (_k = get()) == null ? void 0 : _k.user) == null ? void 0 : _l.name, calls);
const fn = null;
console.log(fn == null ? void 0 : fn(1), typeof (fn == null ? void 0 : fn.call));
const key = "name";
console.log((_m = data == null ? void 0 : data.user) == null ? void 0 : _m[key], (_o = (_n = data.empty) == null ? void 0 : _n.x) != null ? _o : "default");
console.log((_p = data == null ? void 0 : data.user) == null ? true : delete _p.tags, data.user.tags);
class Node {
  constructor(next) {
    this.next = next;
  }
  depth() {
    var _a2, _b2;
    return 1 + ((_b2 = (_a2 = this.next) == null ? void 0 : _a2.depth()) != null ? _b2 : 0);
  }
}
console.log(new Node(new Node(new Node())).depth());
function first(arr) {
  var _a2;
  return (_a2 = arr == null ? void 0 : arr[0]) == null ? void 0 : _a2.value;
}
console.log(first([{ value: 5 }]), first(void 0), first([]));
