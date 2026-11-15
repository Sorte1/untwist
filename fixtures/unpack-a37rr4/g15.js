let [exports] = [1];
console.log(typeof this, (function(){ return typeof this })());
