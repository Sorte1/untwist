function counter(start) {
  let count = start;
  return {
    next() { count += 1; return count; },
    reset() { count = start; return count; },
  };
}
const makers = [];
for (let i = 0; i < 3; i++) makers.push(() => i * 10);
const c = counter(5);
console.log(c.next(), c.next(), c.reset());
console.log(makers.map((make) => make()).join(","));
