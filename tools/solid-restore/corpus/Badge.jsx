export default function Badge(props) {
  return (
    <span class="badge" data-tone={props.tone ?? "plain"}>
      {props.label}
      {props.children}
    </span>
  );
}
