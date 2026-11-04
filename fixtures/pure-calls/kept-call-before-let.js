function probe() {
  let early;
  try {
    early = dec(0);
  } catch (e) {
    early = e.constructor.name;
  }
  let dec = (a) => ["a", "b"][a];
  console.log(early, dec(1));
}
probe();
