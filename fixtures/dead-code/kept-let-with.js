var shade = "outer";
function probe(o) {
  if ("x" === "x") {
    let shade = 1;
    with (o) {
      return shade;
    }
  }
}
console.log(probe({ shade: 2 }));
