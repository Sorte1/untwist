function probe() {
  const dec = (a) => ["a", "b"][a];
  console.log(dec(5), dec(0));
}
probe();
