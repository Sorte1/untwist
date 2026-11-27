function createSignal(v) { return [() => "fake" + v, () => {}]; }
function createEffect(f) { console.log("my effect"); }
function onMount(f) { console.log("my mount"); }
export default function RSig2() {
  const [a] = createSignal(1);
  createEffect(() => console.log("never"));
  onMount(() => console.log("never2"));
  return <div><p>{a()}</p><p>{a}</p></div>;
}
