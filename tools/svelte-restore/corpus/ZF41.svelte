<script>
  let a = $state('A');
  let b = $state('');
  let on = $state(true);
  let items = $state([1, 2]);
  let h = $state('<u>u</u>');
  function t() { a += '!'; b = b ? '' : 'Q'; on = !on; items = [...items, 3]; }
</script>
<button class="go" onclick={t}>go</button>
<ul><li>&#32;x&nbsp;w1  w2{'  '}<b>B</b>&nbsp;{b}  w1  w2{' '}</li></ul>

<pre>{#each items as i}{i} {/each}<i>{a}</i>{@html h} 
 w1  w2{"\t"}{a}{b}&nbsp;</pre>
