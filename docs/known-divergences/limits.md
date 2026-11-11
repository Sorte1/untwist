# Known divergences from the reference deobfuscator: Limits

Untwist refuses an input it cannot process within a bounded stack and memory, rather than let the
process die. Before the parser sees an input, a linear scan (`untwist_core::depth`) bounds how
deeply it nests, how deeply oxc's parser would re-parse parenthesised heads, and how large the
pretty-printed output could grow. An input over any limit is **refused**:

- its text, after the bookmarklet prefix is stripped, is its output, unchanged;
- it gets one note, `pipeline: <why>`, a `Refusal`;
- `Output::depth_refused` (JSON `depth_refused`) lists its 0-based index;
- stderr says `warning: input N was not processed: <why>`, 1-based;
- it takes no part in unpacking: the bundle is built from the other inputs, and with every input
  refused, unpack does not run;
- the CLI exits `3`, unless a parse error, a panic or a lost bundle makes it `1`.

The reference deobfuscator is not known to handle these inputs: no run of it under a memory cap has been made on
them.

**The worker process.** On Unix the CLI runs the pipeline in a worker process. A worker that dies
by a signal while it works on an input makes that input refused, with `processing this input
stopped the worker process, which ran out of stack or memory; it is passed through unchanged`,
and the run is repeated without it (under `-j N`, first once at width 1, so exactly the input that
dies is refused). A death after every input has finished, while the output is assembled, refuses
every input that phase serves (a death in unpack first drops the bundle); a death while the output
is being written exits `1`. Writing makes one more copy of the output, the JSON report's text or
the `--single-file` concatenation, and every printed text is within its input's print budget, so
that copy is bounded by the print budgets of the inputs it holds. A kill for memory counts only when the kernel kills the worker alone. The
worker sets its `oom_score_adj` to 1000 on Linux, so a host-wide OOM kill picks it first; but
under systemd's default `OOMPolicy=stop` (a service or scope with `MemoryMax=`), or in a cgroup
with `memory.oom.group=1`, a memory kill stops the whole unit, the CLI included, and nothing is
refused. Set `OOMPolicy=continue` on such a unit. The `UNTWIST_INTERNAL_*` variables are internal
to that protocol. Windows builds, `UNTWIST_TEST_SUPERVISOR=off` (for measurement) and a host that cannot start a
worker run the pipeline in the CLI's own process. There `--max-memory` and `--timeout` stay hard
stops: at the full memory cap the allocating thread is parked, and past the timeout and its grace
a watchdog takes over; either way the run is repeated in the same process with the inputs that
were in progress refused (`budget-memory` or `budget-time`, as above), under a raised cap and, for
time, with no time left, and the process exits with that run's code. If that repeat runs out of
memory again or takes longer than the time left plus ten seconds, the CLI exits `1` with no
output. A crash (stack overflow, failed allocation) has no repeat in this mode: it ends the run
with no output for any input. The
other `UNTWIST_TEST_*` hooks exist only in builds with the `test-hooks` feature (the README's
"Test hooks").

The in-process depth scan is exact to oxc 0.148 over the oracle grammar; library callers who need the no-crash guarantee for hostile input use process isolation.

## The limits

| Limit | Value on the 1 GiB segment | Refused with |
| --- | --- | --- |
| nesting | `min(100,000, room / (2 × UNIT_STACK_BYTES) − 8)` units | `this input nests deeper than untwist can parse on its stack; it is passed through unchanged` |
| speculative parentheses | 512 nested parenthesised heads oxc may re-parse as arrow parameters | `this input nests parenthesised heads so deeply that parsing it would take memory quadratic in their depth; it is passed through unchanged` |
| speculative pairs | `max(2^17, input bytes / 64)` such heads in all, summed over the input | the same note |
| print size | `min(max(64 MiB, 32 × input bytes), 2^31 − 1)` bytes of estimated pretty output, the estimate being `4 × print weight + 3 × input bytes` | `this input nests statements so deeply that its printed output would exceed the memory budget; it is passed through unchanged` |
| TypeScript syntax | oxc reads some TypeScript in a `.js` program; the scan does not | `this input uses TypeScript syntax that untwist does not read in JavaScript; it is passed through unchanged` |

An internal error while an input is read before its parse (the bookmarklet strip or the scan)
refuses that input alone, with `reading this input before its parse hit an internal error; it is
passed through unchanged`; its text is its output exactly as given.

**The room** is the stack left where the scan runs. Every input runs on a 1 GiB virtual stack
segment (untouched pages cost nothing). A host that refuses that mapping (strict overcommit, a
low `RLIMIT_AS`) gets the next rung of 256 MiB, 64 MiB or 16 MiB, and the nesting limit follows:
about 100,000, 32,700, 8,100 and 1,980 units. A host that maps none of them runs on the calling
thread's own stack. The rung is chosen before the run widens: `-j N` builds its worker pool at the
rung the sequential path gets, or runs sequentially, so `-j 1` and `-j N` refuse the same inputs.

**`UNIT_STACK_BYTES`** is the stack one counted nesting unit may touch: at least 2.3 × the worst
bytes per counted unit measured over every generated depth class (`depth_classes::ALL`) and every
stage that takes no stack hook (parse, semantic, visit, pretty print, reflow), rounded up to a
power of two.

| Build | Worst measured per unit | `UNIT_STACK_BYTES` | Worst at the limit |
| --- | --- | --- | --- |
| release | 1,279 B (arrow heads, parse) | 4,096 | 15.6 % of the room |
| debug | 1,901 B (arrow heads with defaults, parse and reflow) | 8,192 | 11.6 % of the room |

So a debug build refuses earlier: about 65,500 units on the 1 GiB segment.

## What a run may touch

Beyond what the pipeline holds for an input of the same length:

- touched stack ≤ `UNIT_STACK_BYTES` × nesting;
- the pretty text ≤ the print budget, plus reflow's working copies, which are linear in it;
- oxc's speculative re-parse ≤ 2 × 372 B × the pairs limit (the module parse, then the script
  parse into the same arena);
- **across inputs**, with `-j N`, the touched stack of every input being parsed at once is at most
  1 GiB in total: each admitted input reserves `UNIT_STACK_BYTES` × its nesting (at least 1 MiB),
  plus 8 MiB for the texts it decodes, from one process-wide reserve before it is parsed, in input
  order. A decoded text is parsed inside that reservation, so it may nest at most 512 units (256
  in a debug build; a quarter of the 8 MiB, since decoded texts nest up to four deep); a deeper
  one leaves its unit encoded. Whether a unit decodes never depends on `-j` or on the other
  inputs. Heap and print budgets stay
  per input, so with `-j N` up to N inputs hold theirs at once.

- the evaluator's live values during decoding ≤ the heap budget, `max(64 MiB, 16 × input bytes)`
  per input: strings (96 B plus 2 B per unit), arrays (16 B per slot of capacity), objects
  (256 B, plus 48 B per property slot and 64 B for the property map), closures (80 B more) and
  scopes that closures capture or that hold more than 16 bindings (192 B plus 48 B per binding) are charged when allocated and
  credited when freed, and decoding stops partway, leaving the remaining call sites encoded, when
  the live total passes it. One operation can overshoot by what it builds before its check, at
  most one string at the 16 Mi-unit cap or one array at the 1 Mi-slot cap, and while a large array
  or object grows its old and new storage are held together, so peak RSS can reach 1.5 × the charge;
- the decoded text inlined into the output ≤ the smaller of a quarter of the heap budget and the
  print room the input's estimate leaves, counted at its printed size (6 B per unit for strings of
  32 units or fewer, the escaped size above that). The pipeline holds that text about four times
  (the AST, the pretty text, the written output), which the quarter accounts for. Sites past the
  limit stay encoded, with the same note.
- a `JSON.parse('…')` call is replaced by its object literal only when the literal nests at most
  the input's nesting + 8 and its printed size fits what is left of the same print room, counted
  as `3 × length + 4 × (depth of the call + nesting + 1) × (line breaks + 1)`; otherwise the call
  keeps its string. Decoded text and inlined JSON draw on one room per input.

The unpack re-parse of the final texts relies on one premise: a final text nests at most
`2 × nesting(input) + 8`, so it fits the same room. To keep that true, ternary-to-if lifts at most
64 nested levels of a statement ternary chain and leaves the rest as a ternary. It lifts
consequent-nested ternaries in the same visit, up to the same 64 levels, so a chain of n levels
settles in about n / 64 fixpoint iterations instead of n. `a final text nests too deeply to parse again
for unpacking; the inputs are written without a bundle` is the note for a final text that does
not.
