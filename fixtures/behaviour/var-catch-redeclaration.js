function caught() {
  try {
    throw new Error("x");
  } catch (e) {
    var e = 2;
    return e;
  }
}
console.log(caught());
