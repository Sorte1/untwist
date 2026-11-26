import { createSignal } from "solid-js";

export default function Field() {
  const [text, setText] = createSignal("start");
  const [agree, setAgree] = createSignal(false);
  return (
    <form onSubmit={(e) => e.preventDefault()}>
      <input class="name" value={text()} onInput={(e) => setText(e.currentTarget.value)} />
      <input class="agree" type="checkbox" checked={agree()} onChange={(e) => setAgree(e.currentTarget.checked)} />
      <textarea value={text()} />
      <p>
        {text()} {agree() ? "yes" : "no"}
      </p>
    </form>
  );
}
