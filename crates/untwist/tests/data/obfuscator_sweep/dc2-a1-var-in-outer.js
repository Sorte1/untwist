function g(c) {
  const r = [];
  switch (c) {
    case 1:
      var y;
      if ("a" === "a") { class y {} r.push(typeof y, y.name); }
      if ("a" === "a") { let z = function () {}; r.push(z.name); } else { var z = 1; }
      var z;
      break;
  }
  return r.join() + typeof y + typeof z;
}
console.log(g(1));
