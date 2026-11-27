<script>
  let a = $state('A');
  let b = $state('');
  let on = $state(true);
  let items = $state([1, 2]);
  let h = $state('<u>u</u>');
  function t() { a += '!'; b = b ? '' : 'Q'; on = !on; items = [...items, 3]; }
</script>
<button class="go" onclick={t}>go</button>
<pre>{#if on} Y {:else} N {/if} <b>B</b>x&#10;w1  w2</pre> <p>{"\t"}{a}{' '}</p>
