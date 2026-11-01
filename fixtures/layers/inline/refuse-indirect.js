function site() {
  var near = "local";
  console.log("indirect", (0, eval)("typeof near"));
}
site();
