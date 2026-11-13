var count = 0;
function bump() {
  count += 1;
  return count;
}
var selfIsExports = this === module.exports;
if (count === 0) {
  function blockFn() {
    return typeof bump;
  }
}
module.exports = { bump: bump, selfIsExports: selfIsExports, blockFn: blockFn };
