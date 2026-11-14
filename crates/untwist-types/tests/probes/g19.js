(function () {
  function outer(a) {
    {
      function a() {}
    }
    console.log(typeof a);
  }
  outer(1);
  function h(b) {
    try { throw "s"; } catch (b) { var b = true; }
    console.log(typeof b);
  }
  h(1);
  function z(c) {
    for (var c in { key: 1 }) {}
    console.log(typeof c);
  }
  z(1);
  function y(d) {
    for (var d of [true]) {}
    console.log(typeof d);
  }
  y(1);
  function x(e) {
    var [e] = ["s"];
    console.log(typeof e);
  }
  x(1);
})();
