const wait = (value, ms) => new Promise((resolve) => setTimeout(() => resolve(value), ms));
async function total(items) {
  let sum = 0;
  for (const item of items) sum += await wait(item, 1);
  return sum;
}
async function main() {
  const values = await Promise.all([wait("a", 3), wait("b", 1)]);
  console.log("values", values.join("-"));
  console.log("total", await total([1, 2, 3]));
  try {
    await Promise.reject(new Error("nope"));
  } catch (error) {
    console.log("caught", error.message);
  }
}
main();
