Hand-edited webpack 5.112 inputs, not tool output. Each case keeps webpack's stock
`target: node` (or `web`) runtime from `fixtures/webpack-variants` and replaces the entry,
a chunk file or the code around the bundle with a shape the real runtime treats differently
from a merged tree. `EXPECT` says which cases must be refused and which must run with the
same output as the input under node.
