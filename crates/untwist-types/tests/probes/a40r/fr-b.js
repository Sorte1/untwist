function helper() {
  if (!helper.done) { helper.done = true; helper.caller("str"); }
}
