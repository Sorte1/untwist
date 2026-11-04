function probe(k) {
  switch (k) {
    case 0:
      let dec = (a) => ["a"][a];
    case 1:
      try {
        return dec(0);
      } catch (e) {
        return e.constructor.name;
      }
  }
}
console.log(probe(0), probe(1));
