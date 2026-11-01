var holder = {
  check: function () {
    var inner = function () {
      return this === holder;
    };
    return inner();
  },
};
console.log(holder.check());
