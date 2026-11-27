<script>
  let a = $state('x');
</script>
<button onclick={() => (a += 'y')}>b</button>
<pre>
  first
    {a}
</pre>
<pre>

two</pre>
<pre>static
  only </pre>
<textarea>
 lead {a}</textarea>
<p>
  {a}
  <b> bold </b>
  tail
</p>
<p>x{' '}{a}{'  '}y &nbsp; z</p>
<span> </span><em>{' '}</em>
<p>{a} </p><p> {a}</p>
<div>a<!-- c -->b</div>
