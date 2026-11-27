<script>
  let a = $state('A');
  let b = $state('');
  let on = $state(true);
  let items = $state([1, 2]);
  let h = $state('<u>u</u>');
  function t() { a += '!'; b = b ? '' : 'Q'; on = !on; items = [...items, 3]; }
</script>
<button class="go" onclick={t}>go</button>
<span>{' '}&amp;&nbsp;<em></em>{'  '}</span>

<pre>{"\t"}w1  w2<em></em>
<em></em>  {#if on} Y {:else} N {/if}{a}{b}{'\n'}</pre>
