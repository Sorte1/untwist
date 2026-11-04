function factorial(n) {
  return n <= 1 ? 1 : n * factorial(n - 1);
}

function compose(first, second) {
  return function (value) {
    return second(first(value));
  };
}

function makeCounter(start) {
  var count = start;
  return {
    next: function () {
      count += 1;
      return count;
    },
    reset: function () {
      count = start;
      return count;
    },
  };
}

const double = (x) => x * 2;
const increment = (x) => x + 1;
const pipeline = compose(double, increment);

function sum(...values) {
  return values.reduce((total, value) => total + value, 0);
}

function greetAll(greeting = "hi", ...names) {
  return names.map((name) => greeting + " " + name).join("; ");
}

const counter = makeCounter(10);
counter.next();
counter.next();
console.log("factorial", factorial(7));
console.log("pipeline", pipeline(20));
console.log("counter", counter.next(), counter.reset());
console.log("sum", sum(4, 8, 15, 16, 23, 42));
console.log("greet", greetAll(undefined, "north", "south"));
console.log("apply", Math.max.apply(null, [3, 9, 1]));
