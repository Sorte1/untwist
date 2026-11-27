const list = [];
function add(x) { list.push(x); }
add(1);
add(2);
export default function ZModFn() { return <p>{list.join()}</p>; }
