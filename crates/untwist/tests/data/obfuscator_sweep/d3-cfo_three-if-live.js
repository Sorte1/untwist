function arr() {
  const a = ["log", "hello", "world", "abcde", "length", "push", "yes", "changed", "klmno", "1|0|2", "pqrst"];
  arr = function () {
    return a;
  };
  return arr();
}
function dec(i, k) {
  i = i - 100;
  const a = arr();
  let s = a[i];
  return s;
}
function leak(v) {
  v.klmno = dec(107);
}

function main(x) {
  const o = {
    abcde: function (a, b) {
      return a(b);
    },
    fghij: function (a, b) {
      return a === b;
    },
    pqrst: function (a, b) {
      return a(b);
    },
    klmno: "yes"
  };
  
  const g = { zzzzz: function (a, b, c) { return a(b, c); }, yyyyy: function (a, b) { return a + b; } };
  if (o.fghij(o.abcde(dec, 101), "hellx")) {
    console[dec(100)](o.klmno, x);
    
  } else {
    g.zzzzz(leak, o, 1);
  }
  
  console[dec(100)](o.klmno, dec(102));
}
main(1);
