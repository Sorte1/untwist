function decoyBody() {
  return "decoy text";
}
const decoyData = ["read through the module text"];
console.log(arguments.callee.toString().includes("decoyBody"), arguments.callee.toString().includes("decoyData"));
