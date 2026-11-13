const data = { user: { name: "ann", tags: ["a"], greet() { return "hi " + this.name; } }, empty: null };
let calls = 0;
const get = () => {
  calls++;
  return data;
};
console.log(data.user?.name, data.empty?.name, data.missing?.deep.deeper);
console.log(data.user?.tags?.[0], data.empty?.[0], data.user.greet?.(), data.user.nope?.());
console.log(get()?.user?.name, calls);
const fn = null;
console.log(fn?.(1), typeof fn?.call);
const key = "name";
console.log(data?.user?.[key], (data.empty)?.x ?? "default");
console.log(delete data?.user?.tags, data.user.tags);
class Node {
  constructor(next) {
    this.next = next;
  }
  depth() {
    return 1 + (this.next?.depth() ?? 0);
  }
}
console.log(new Node(new Node(new Node())).depth());
function first(arr) {
  return arr?.[0]?.value;
}
console.log(first([{ value: 5 }]), first(undefined), first([]));
