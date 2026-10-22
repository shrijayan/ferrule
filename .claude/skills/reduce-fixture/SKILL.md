---

name: reduce-fixture
description: Reduce a failing parity, corpus or user bug to a minimal fixture and pin it with a failing test before fixing. Use whenever fixing a bug.
---

# Reduce to a minimal fixture (test-first bug fixing)

1. **Reproduce.** Run the failing command (e.g. `mise run parity -- <oracle>`) and note the FER code and spec/config pointer.
2. **Find the owner.** Converter (config/extensions), python-plugin (emitted files), runtime (behavior), migrate (adopt/verify/merge), or Forge (generic; file upstream).
3. **Shrink.** Copy the spec and `stainless.yml` into a scratch dir. Delete paths, schemas, resources and config keys one at a time, re-running after each, until removing anything makes the failure disappear.
4. **Name it.** Put the result under the owner's fixtures (e.g. `python-plugin/test/fixtures/<short-name>/`) with a one-line README saying what it proves and where it came from. Only public, licence-compatible inputs.
5. **Pin.** Add a test that fails for the right reason. Commit it alone, signed off: `test(<part>): reproduce <bug>`.
6. **Fix** in the owning part, then commit `fix(<part>): ...`. If the bug is Forge's, open an upstream issue or PR and mark the test as expected-failure with the link.
7. Run `mise run check`.
