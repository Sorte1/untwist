function site() {
  eval("var made = 1");
  console.log("eval-declared var", delete made, typeof made);
}
site();
