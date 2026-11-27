function next() { next.n = (next.n || 0) + 1; return next.n; }
function Alpha() { return <b>{next()}</b>; }
function Beta() { return <i>{next()}</i>; }
export default function ZFnState() { return <div><Alpha /><Beta />{next()}</div>; }
