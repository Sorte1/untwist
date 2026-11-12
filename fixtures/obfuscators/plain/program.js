var palette = ["crimson", "teal", "amber", "violet"];
var settings = { width: 640, height: 360, title: "untwist sample", tags: ["alpha", "beta"] };

function Counter(start) {
  this.value = start;
}
Counter.prototype.bump = function (step) {
  this.value += step;
  return this;
};

function describe(item, index) {
  return index + ":" + item.toUpperCase().split("").reverse().join("");
}

function area(box) {
  return box.width * box.height;
}

function fizz(n) {
  var out = [];
  for (var i = 1; i <= n; i++) {
    out.push(i % 15 === 0 ? "FizzBuzz" : i % 3 === 0 ? "Fizz" : i % 5 === 0 ? "Buzz" : String(i));
  }
  return out.join(",");
}

var counter = new Counter(3).bump(4).bump(5);
console.log("palette", palette.map(describe).join(" "));
console.log("area", area(settings), settings.title, settings.tags.length);
console.log("counter", counter.value);
console.log("fizz", fizz(16));
console.log("json", JSON.stringify({ keys: Object.keys(settings), sum: palette.reduce(function (a, c) { return a + c.length; }, 0) }));
try {
  null.missing;
} catch (error) {
  console.log("caught", error instanceof TypeError);
}
