var _0x3f1a = ["\x76\x69\x6F\x6C\x65\x74", "\x75\x6E\x74\x77\x69\x73\x74\x20\x73\x61\x6D\x70\x6C\x65", "\x61\x6C\x70\x68\x61", "\x62\x65\x74\x61", "\x76\x61\x6C\x75\x65", "\x70\x72\x6F\x74\x6F\x74\x79\x70\x65", "\x62\x75\x6D\x70", "\x3A", "\x74\x6F\x55\x70\x70\x65\x72\x43\x61\x73\x65", "\x73\x70\x6C\x69\x74", "", "\x72\x65\x76\x65\x72\x73\x65", "\x6A\x6F\x69\x6E", "\x77\x69\x64\x74\x68", "\x68\x65\x69\x67\x68\x74", "\x70\x75\x73\x68", "\x46\x69\x7A\x7A\x42\x75\x7A\x7A", "\x46\x69\x7A\x7A", "\x42\x75\x7A\x7A", "\x2C", "\x6C\x6F\x67", "\x70\x61\x6C\x65\x74\x74\x65", "\x6D\x61\x70", "\x20", "\x61\x72\x65\x61", "\x74\x69\x74\x6C\x65", "\x74\x61\x67\x73", "\x6C\x65\x6E\x67\x74\x68", "\x63\x6F\x75\x6E\x74\x65\x72", "\x66\x69\x7A\x7A", "\x6A\x73\x6F\x6E", "\x73\x74\x72\x69\x6E\x67\x69\x66\x79", "\x6B\x65\x79\x73", "\x72\x65\x64\x75\x63\x65", "\x6D\x69\x73\x73\x69\x6E\x67", "\x63\x61\x75\x67\x68\x74", "\x63\x72\x69\x6D\x73\x6F\x6E", "\x74\x65\x61\x6C", "\x61\x6D\x62\x65\x72"];
(function (list, times) { while (times--) { list.unshift(list.pop()); } })(_0x3f1a, 3);
var palette = [_0x3f1a[0], _0x3f1a[1], _0x3f1a[2], _0x3f1a[3]];
var settings = { width: 640, height: 360, title: _0x3f1a[4], tags: [_0x3f1a[5], _0x3f1a[6]] };

function Counter(start) {
  this[_0x3f1a[7]] = start;
}
Counter[_0x3f1a[8]][_0x3f1a[9]] = function (step) {
  this[_0x3f1a[7]] += step;
  return this;
};

function describe(item, index) {
  return index + _0x3f1a[10] + item[_0x3f1a[11]]()[_0x3f1a[12]](_0x3f1a[13])[_0x3f1a[14]]()[_0x3f1a[15]](_0x3f1a[13]);
}

function area(box) {
  return box[_0x3f1a[16]] * box[_0x3f1a[17]];
}

function fizz(n) {
  var out = [];
  for (var i = 1; i <= n; i++) {
    out[_0x3f1a[18]](i % 15 === 0 ? _0x3f1a[19] : i % 3 === 0 ? _0x3f1a[20] : i % 5 === 0 ? _0x3f1a[21] : String(i));
  }
  return out[_0x3f1a[15]](_0x3f1a[22]);
}

var counter = new Counter(3)[_0x3f1a[9]](4)[_0x3f1a[9]](5);
console[_0x3f1a[23]](_0x3f1a[24], palette[_0x3f1a[25]](describe)[_0x3f1a[15]](_0x3f1a[26]));
console[_0x3f1a[23]](_0x3f1a[27], area(settings), settings[_0x3f1a[28]], settings[_0x3f1a[29]][_0x3f1a[30]]);
console[_0x3f1a[23]](_0x3f1a[31], counter[_0x3f1a[7]]);
console[_0x3f1a[23]](_0x3f1a[32], fizz(16));
console[_0x3f1a[23]](_0x3f1a[33], JSON[_0x3f1a[34]]({ keys: Object[_0x3f1a[35]](settings), sum: palette[_0x3f1a[36]](function (a, c) { return a + c[_0x3f1a[30]]; }, 0) }));
try {
  null[_0x3f1a[37]];
} catch (error) {
  console[_0x3f1a[23]](_0x3f1a[38], error instanceof TypeError);
}
