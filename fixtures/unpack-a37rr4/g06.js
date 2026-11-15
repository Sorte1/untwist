class require {}
console.log(typeof this, (function(){ return typeof this })());
