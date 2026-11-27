function decoyBody() {
  return 7;
}
const read = () => typeof arguments[2];
console.log(read(), typeof decoyBody);
