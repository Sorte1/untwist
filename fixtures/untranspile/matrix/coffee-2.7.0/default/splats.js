(function() {
  var a, b, k, n, nums, rest, squares, sum, why, x,
    hasProp = {}.hasOwnProperty,
    indexOf = [].indexOf;

  sum = function(first, ...rest) {
    return first + rest.reduce((function(a, b) {
      return a + b;
    }), 0);
  };

  nums = [1, 2, 3];

  console.log(sum(...nums), sum(0, ...nums), [...nums, 4].join(","));

  [a, b, ...rest] = [1, 2, 3, 4];

  ({
    x,
    y: why = 5
  } = {
    x: 1
  });

  console.log(a, b, rest.join(","), x, why);

  squares = (function() {
    var i, len, results;
    results = [];
    for (i = 0, len = nums.length; i < len; i++) {
      n = nums[i];
      if (n !== 2) {
        results.push(n * n);
      }
    }
    return results;
  })();

  console.log(squares.join(","), ((function() {
    var ref, results;
    ref = {
      p: 1,
      q: 2
    };
    results = [];
    for (k in ref) {
      if (!hasProp.call(ref, k)) continue;
      results.push(k);
    }
    return results;
  })()).join(","));

  console.log(2 ** 10, `${a} and ${b}`, indexOf.call(nums, 3) >= 0);

}).call(this);
