import * as me from './program.mjs';
function c() {
  var o = ['TyxkK', 'hello', 'log', 'world'];
  c = function () { return o; };
  return c();
}
function d(a, b) {
  var e = c();
  return (d = function (f, g) {
    f = f - 0x10;
    var h = e[f];
    return h;
  }), d(a, b);
}
var z = d;
console.log(z(0x11));
var t0 = d;
export let t = t0;
console.log(z(0x13));
var u0 = d;
export const u = u0;
console.log(me.t === me.u);
