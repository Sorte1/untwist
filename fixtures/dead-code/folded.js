function probe(flag) {
  var out = [];
  if ("a" === "a") {
    let local = flag ? 1 : 2;
    out.push(local);
  } else {
    var spare = 3;
  }
  out.push(typeof spare, "b" === "c" ? "no" : "yes");
  return out.join(",");
}
console.log(probe(true));
