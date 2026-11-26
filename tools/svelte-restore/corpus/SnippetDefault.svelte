<script>
  let count = $state(1);
</script>

{#snippet tag(text, n = 5)}
  <b>{text}:{n}</b>
{/snippet}

{@render tag('a')}
{@render tag('b', count)}
<button onclick={() => count++}>more</button>
