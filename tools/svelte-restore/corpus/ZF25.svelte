<script>
  let a = $state('A');
  let b = $state('');
  let on = $state(true);
  let items = $state([1, 2]);
  let h = $state('<u>u</u>');
  function t() { a += '!'; b = b ? '' : 'Q'; on = !on; items = [...items, 3]; }
</script>
<button class="go" onclick={t}>go</button>
<ul><li>{'\n'}&amp;{' '}&amp;{'  '}</li></ul><pre>x&nbsp;{a}{b}{' '}&amp;{'\n'}<em></em>{#if on}Y{/if}&nbsp;</pre>
