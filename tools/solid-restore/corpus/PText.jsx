import { createSignal } from "solid-js";
export default function PText() {
  const [n, setN] = createSignal(1);
  return (
    <div>
      <p textContent={n()} />
      <p textContent={/*@once*/ "s" + n()} />
      <p>{n()}</p>
      <p>pre {n()}</p>
      <p>{n()} post</p>
      <p>{n()}{n() * 2}</p>
      <p>{`tpl ${n()}`}</p>
      <p>{"lit"}{1}{true}{null}{undefined}{[1, 2]}</p>
      <p>{n() + ""}{n().toString()}{String(n())}</p>
      <button onClick={() => setN(n() + 1)}>inc</button>
    </div>
  );
}
