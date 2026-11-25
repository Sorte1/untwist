<script>
  let numbers = $state([1, 2, 3]);
  let total = $derived.by(() => {
    let sum = 0;
    for (const n of numbers) sum += n;
    return sum;
  });
</script>

<p>{numbers.join(' + ')} = {total}</p>
<button onclick={() => numbers.push(numbers.length + 1)}>more</button>
