<script>
  let { label = 'row' } = $props();
  let count = $state(1);
</script>

{#snippet cell(text, n)}
  <td>{text} #{n}</td>
{/snippet}

<table>
  <tbody>
    <tr>
      {@render cell(label, count)}
      {@render cell('fixed', 2)}
    </tr>
  </tbody>
</table>
<button onclick={() => count++}>more</button>
