(function () {
  function f() {
    return 1;
  }
  {
    function f() {
      return "s";
    }
  }
  const r = f();
  console.log(typeof r);
  const g = function () {
    return 2;
  };
  if (r) {
    function g() {
      return null;
    }
  }
  var h = function () {
    return 3;
  };
  switch (r) {
    case "s":
      function h() {
        return "t";
      }
  }
  const s = h();
  console.log(typeof s);
})();
