const mk = (k) => { console.log(k); return () => {}; };
export default function ZEv2() {
  return <div><button onClick={mk("1")} onInput={mk("2")} on:foo={mk("3")} onMouseEnter={mk("4")} oncapture:bar={mk("5")} onKeyDown={[mk("6"), 1]}>b</button></div>;
}
