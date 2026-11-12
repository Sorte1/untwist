var count = 0;
module.exports = function log(line) {
  count += 1;
  console.log(count + " " + line);
};
