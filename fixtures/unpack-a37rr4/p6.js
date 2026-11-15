class A extends (await Object) {}
console.log(typeof this, (function(){ return typeof this })());
