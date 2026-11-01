function site() {
  eval("'use strict'; var inner = 1; console.log('strict payload', typeof inner)");
  console.log("after", typeof inner);
}
site();
