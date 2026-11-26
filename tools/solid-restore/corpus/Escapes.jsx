export default function Escapes(props) {
  return (
    <div title={'say "hi"'} data-amp="a&b">
      <p>a &lt; b &amp;&amp; c &gt; d</p>
      <p>{"{braces}"} and {"<tag>"}</p>
      <p>quote's "double" `tick` ${"{"}dollar</p>
      <p>&nbsp;spaced&nbsp;</p>
      <p>{props.text}</p>
      <p>{" "}</p>
    </div>
  );
}
