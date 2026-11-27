const list = [];
class Reg { static { list.push("s"); } static made = list.push("m"); }
export default function YModClass() { return <p>{list.join()}</p>; }
