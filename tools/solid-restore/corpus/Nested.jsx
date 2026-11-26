import { createSignal } from "solid-js";

export default function Nested(props) {
  const [n, setN] = createSignal(0);
  return (
    <article>
      <header>
        <h2>{props.title}</h2>
        <small>
          by {props.author} on <time>{props.date}</time>
        </small>
      </header>
      <div class="body">
        <p>
          first {n()} middle {n() * 2} last
        </p>
        <p>
          <span>{n()}</span>
          <span>{n() + 1}</span>
        </p>
      </div>
      <footer>
        <button onClick={() => setN(n() + 1)}>next</button>
      </footer>
    </article>
  );
}
