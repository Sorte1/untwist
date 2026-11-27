export default function ZSigAfter() {
  const [n] = createSignal(4);
  onMount(() => console.log("mounted-arg"));
  return <div><p>{n()}</p><p>{For({ each: [1, 2, 3] })}</p></div>;
}
function createSignal(v) { console.log("mine"); return [() => "mine:" + v, () => {}]; }
function onMount(f) { console.log("my-mount"); }
function For(props) { return props.each.length + " items"; }
