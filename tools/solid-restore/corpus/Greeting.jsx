export default function Greeting(props) {
  return (
    <p class="greeting" title={props.title}>
      Hello, {props.name}!
    </p>
  );
}
