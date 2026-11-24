function c() {
  var o = ['TyxkK', 'hello', 'log', 'abc', 'def', 'x', 'z', '__proto__'];
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
function show(o){try{return Object.keys(o).join()+'|'+(Object.getPrototypeOf(o)===Object.prototype)+'|'+typeof o.__proto__+'|'+JSON.stringify(o)}catch(e){return 'E'}}
(function(){ var __proto__ = 0x11; var u; ({__proto__: u} = {__proto__}); console.log(d(__proto__), u); })();
