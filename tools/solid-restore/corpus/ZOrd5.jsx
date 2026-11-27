const mk = (k) => { console.log(k); return () => {}; };
export default function ZOrd5() {
  let el;
  return <div><p ref={el} onClick={mk("c")} onInput={mk("i")}>x</p></div>;
}
