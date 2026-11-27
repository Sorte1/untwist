<script>
  let a = $state('A');
  let b = $state('');
  let on = $state(true);
  let items = $state([1, 2]);
  let h = $state('<u>u</u>');
  function t() { a += '!'; b = b ? '' : 'Q'; on = !on; items = [...items, 3]; }
</script>
<button class="go" onclick={t}>go</button>
<pre>  &amp;	&amp; 
 {a}{b}{'  '}<span> s </span> w1  w2{"\t"}</pre> <pre>w1  w2&#10;<b>B</b>	w1  w2{b}	</pre> <pre>  <b>B</b> 
 w1  w2{#if on}Y{/if}x</pre>
