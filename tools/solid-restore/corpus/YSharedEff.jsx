const a = [];
const b = { list: a };
a.push(1, 2);
function Alpha() { return <b>{b.list.length}</b>; }
function Beta() { return <i>{b.list.join()}</i>; }
export default function YSharedEff() { return <div><Alpha /><Beta /></div>; }
