function makeCounter() { let n = 0; return { get: () => n, inc: () => ++n }; }
const { get, inc } = makeCounter();
function Alpha() { inc(); return <b>a</b>; }
function Beta() { return <i>{get()}</i>; }
export default function YDestrObj() { return <div><Alpha /><Beta /></div>; }
