function helper() {
  if (!helper.done) {
    helper.done = true;
    with (helper) {
      caller("str");
    }
  }
}
