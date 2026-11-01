var named = function inner() {
  return 2;
};

var Named = class Inner {};

var result = function inner() {
  return 2;
}();

var user = getUser();
function getUser() {
  return 1;
}

var a = b;
var b = 1;
