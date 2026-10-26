---

title: Architecture
description: Five stages; Forge is stage two.
---

**Inputs:** OpenAPI spec · `stainless.yml` · `.ferrule/patches/` · previous release
**Outputs:** an SDK per language · `api.md` · `.stats.yml` · agent context · `ferrule.lock` · verify report

| Stage      | What happens                                                                                                              | Part                          |
| ---------- | ------------------------------------------------------------------------------------------------------------------------- | ----------------------------- |
| 01 Convert | Validate `stainless.yml`, apply editions and `openapi.transforms`, write an annotated spec with `x-ferrule-*` extensions. | converter (Rust)              |
| 02 Forge   | Forge, a Cargo dependency pinned to one commit, parses the annotated spec and runs the plugin in the same process.       | dependency                    |
| 03 Emit    | An adapter maps Forge data to our model; the language's emitters write the SDK files.                                     | a plugin per language (Rust)  |
| 04 Finish  | Copy runtime, format (ruff for Python), merge custom code onto the generated branch.                                      | migrate (Python)              |
| 05 Verify  | API diff, type-check, tests against the mock, semver verdict.                                                             | migrate (Python)              |

Stages 01 to 03 are one native Rust binary, `ferrule-gen` (`crates/ferrule-gen`), 8 MB with
Forge built in: `ferrule-gen convert` writes the annotated spec and `ferrule-gen generate` runs
Forge with the plugin. The `ferrule` CLI calls it for `generate` and `adopt`; no Node or
WebAssembly build is involved (ADR 0014). Ruff formatting runs separately. The output is
byte-identical on every run.

## One model, one plugin per language

Every language's plugin is on `main`. Python's is `crates/python-plugin`; the other seven are built
on ADR 0017:

- `crates/sdk-model` holds the language-neutral model and its adapter from Forge. Names are word
  lists, never a language's spelling.
- One plugin crate per language: `ts-plugin`, `go-plugin`, `jvm-plugin` for Java and Kotlin,
  `ruby-plugin`, `php-plugin` and `csharp-plugin`. `ferrule-gen --target <lang>` picks one.
- Each language's runtime is copied from Apache-2.0 Stainless SDKs.

Python keeps its own adapter for now and moves onto the shared model later, as ADR 0017 plans.

```rust
// crates/python-plugin/src/plugin.rs: the plugin entry, a native forge::Plugin.
// Forge types reach our model only through crates/python-plugin/src/adapter/.
impl forge::Plugin for PythonPlugin {
    type File = OutputFile;

    fn generate(&self, forge: &Forge) -> Result<impl Iterator<Item = OutputFile>, forge::Err> {
        let files = self.files(forge)?; // adapter: Forge -> SdkClient, then emitters -> files
        Ok(files.into_iter().map(|(path, content)| OutputFile { path, content }))
    }
}
```

## Plugin limits we design around

The plugin is pure: it reads no files, network or clock, so ruff formatting and checks run after
Forge. Forge may run groups in parallel, so the plugin keeps no state and writes every file from
the one `generate` call; group order cannot change the output. The plugin sees only the spec, so
all `stainless.yml` settings travel as extensions or overlays (see ADR 0003).
