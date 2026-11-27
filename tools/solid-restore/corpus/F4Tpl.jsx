export default function F4Tpl(props) {
  const s = `x
y`;
  return (
    <div class="some-long-class-name-to-force-wrapping" data-a="another-long-attribute-value-here">
      <p title={`a
b ${props.x}`} data-long="another-long-attribute-value-here-to-wrap-the-line-for-sure">{`c
d ${props.x}`}{s}</p>
      <pre>{props.x + `
  two`}</pre>
    </div>
  );
}
