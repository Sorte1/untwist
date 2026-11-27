<script>
  let a = $state('A');
  let b = $state('');
  let on = $state(true);
  let items = $state([1, 2]);
  let h = $state('<u>u</u>');
  function t() { a += '!'; b = b ? '' : 'Q'; on = !on; items = [...items, 3]; }
</script>
<button class="go" onclick={t}>go</button>
<pre>{'  '}w1  w2
{#if on} Y {:else} N {/if}</pre>
<table><tbody><tr><td>&#32;<span> s </span>
<em></em>{#each items as i}{i} {/each}&nbsp;</td></tr></tbody></table>
