---

name: reduce-fixture
description: Reduce a failing parity, corpus or user bug to a minimal fixture and pin it with a failing test before fixing. Use whenever fixing a bug.
---

# Reduce to a minimal fixture (test-first bug fixing)

1. **Reproduce.** Run the failing command (e.g. `mise run parity -- <oracle>`) and note the FER code and spec/config pointer.
2. **Find the owner.** Converter (`crates/converter`: config/extensions), python-plugin (`crates/python-plugin`: emitted files), runtime (behavior), migrate (adopt/verify/merge), or Forge (generic; file upstream).
3. **Shrink.** Copy the spec and `stainless.yml` into a scratch dir. Delete paths, schemas, resources and config keys one at a time, re-running after each, until removing anything makes the failure disappear.
4. **Name it.** Put the result under the owner's fixtures and say in one line what it proves and where it came from. Only public, licence-compatible inputs.
   - python-plugin: an annotated spec at `crates/python-plugin/tests/fixtures/<short-name>.json` (produce it with `ferrule-gen convert <spec> <config> -o <short-name>.json`). The golden test compares it with `crates/python-plugin/tests/golden/<short-name>/`; a new or changed golden tree is approved by a human.
   - converter: a Rust test case in `crates/converter/tests/` (for example `transforms.rs` or `extensions.rs`) with the reduced spec and config inline, asserting the output or the FER diagnostic.
   - runtime, migrate: a pytest case under that part's `tests/`.
5. **Pin.** Add a test that fails for the right reason. Commit it alone, signed off: `test(<part>): reproduce <bug>`.
6. **Fix** in the owning part, then commit `fix(<part>): ...`. If the bug is Forge's, open an upstream issue or PR and mark the test as expected-failure with the link.
7. Run `mise run check`.
