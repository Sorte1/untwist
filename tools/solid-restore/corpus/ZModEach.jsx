const list = [];
const add = (x) => list.push(x);
[1, 2, 3].forEach(add);
export default function ZModEach() { return <p>{list.join()}</p>; }
