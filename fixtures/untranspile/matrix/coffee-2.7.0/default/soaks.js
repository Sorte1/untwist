(function() {
  var base, counter, data, ref, ref1, ref2, ref3, ref4, value;

  data = {
    user: {
      name: "ann",
      greet: function() {
        return "hi";
      }
    },
    empty: null
  };

  console.log((ref = data.user) != null ? ref.name : void 0, (ref1 = data.empty) != null ? ref1.name : void 0, typeof (base = data.user).greet === "function" ? base.greet() : void 0, (ref2 = data.missing) != null ? ref2.deep : void 0);

  value = (ref3 = data.empty) != null ? ref3 : "fallback";

  console.log(value, (ref4 = data.user.name) != null ? ref4 : "x");

  console.log(typeof undeclaredThing === "undefined", typeof undeclaredThing !== "undefined" && undeclaredThing !== null);

  counter = {
    count: 0,
    start: function() {
      return [1, 2, 3].forEach((n) => {
        this.count += n;
        return this.count;
      });
    }
  };

  console.log(counter.start());

}).call(this);
