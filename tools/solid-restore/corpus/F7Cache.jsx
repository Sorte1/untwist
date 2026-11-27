const seen = [];
function remember(k) { seen.push(k); return seen.length; }
function Alpha() { return <b>{remember("a")}</b>; }
function Beta() { return <i>{remember("b")}</i>; }
export default function F7Cache() {
  return <div><Alpha /><Beta />{remember("root")}</div>;
}
