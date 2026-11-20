function arr() {
  const a = ["log", "hello", "world", "changed"];
  arr = function () {
    return a;
  };
  return arr();
}
function dec(i) {
  i = i - 100;
  const a = arr();
  let s = a[i];
  return s;
}
function leak(v) {
  v.klmno = dec(103);
}
const o = { klmno: "outer" };
function main(x) {
  const o = {
    abcde: function (a, b) {
      return a(b);
    },
    fghij: function (a, b) {
      return a === b;
    },
    klmno: "yes"
  };
  if (o.fghij(o.abcde(dec, 101), x)) {
    o.abcde(leak, o);
  }
  console[dec(100)](o.klmno, dec(102));
}
main("hello");
console.log(o.klmno);
