# Known divergences from the reference deobfuscator: Plugins

Untwist's plugin hooks (`Options::plugins`) have no differential against the reference deobfuscator; their
placement is verified in-process by `crates/untwist/tests/plugins.rs`. The item below is deliberate,
not a bug.

- **`AfterUnpack` runs over each extracted module and is skipped when there is no bundle.** The
  hook runs over every extracted module's own program once the unpacker has finished with it, and
  is skipped, with no `plugin:after-unpack` row in the report, when unpacking is off or no bundle
  is detected. The reference deobfuscator runs `afterUnpack` over the whole-file AST whenever the plugin is
  registered, whether or not `unpackAST` found anything (`src/index.ts:217`). Untwist prints each
  program before unpacking and clones module bodies out of the bundle, so a hook over a non-bundle
  would run after the program had already been printed and would need a re-print pass; running it
  per module is what the hook is for.

