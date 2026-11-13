var app = (function () {
  'use strict';

  var props$1 = {};

  var count = 0;
  var log = function log(line) {
    count += 1;
    console.log(count + " " + line);
  };

  log("props eval");
  props$1.one = 1;
  props$1.two = function () {
    return 2;
  };

  const props = props$1;
  log("mixed " + props.one);
  var mixed = "mixed";

  return mixed;

})();
