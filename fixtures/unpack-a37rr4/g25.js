let exports = 0; delete exports.x;
console.log(typeof this, (function(){ return typeof this })());
