function Show(props) { return <b class="mine">custom:{props.when ? "T" : "F"}:{props.children}</b>; }
function For(props) { return <ol>{props.each.length} items</ol>; }
export default function RShow(props) {
  return <div><Show when={props.flag}>kid</Show><For each={[1, 2, 3]}>{(x) => <li>{x}</li>}</For></div>;
}
