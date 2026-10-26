# ferrule-ruby-plugin

The Ruby plugin: the neutral model of `ferrule-sdk-model` to a gem in Stainless's Ruby layout,
with `lib/` (the code), `rbi/` (Sorbet) and `sig/` (RBS) views of every model and resource. It
never imports Forge; the model comes from `ferrule_sdk_model::forge_to_model` (`tests/boundary.rs`).

```text
annotated spec --sdk-model adapter--> model::Client --for_target("ruby")--> namer + emitters
  --> generated files + runtime options --ferrule_runtime (runtime/ruby)--> gem
  --> rubocop -a (.rb), stree write (.rbi), stree write --plugin=rbs (.rbs)
```

- `naming.rs`: class, constant and method names with Stainless's initialisms and
  `x-ferrule-casings`, and the RBS type aliases that dodge reserved words (`name_`, `type_`).
- `types.rs`: the one type mapper, from the model's types to each view's spelling (the
  `.rb` converter, YARD, Sorbet input and output forms, RBS).
- `tree.rs`: the gem's declarations (classes, enums, unions, converter constants) built from
  the model; field order, initializer order and the input/output role of each class.
- `doc.rs`, `printer.rs`: a Wadler printer for the generator's 110-column `.rb` statements and
  the 80-column markdown reflow of doc comments.
- `emit/`: models and resources in the three views, the client, `lib/<gem>.rb` (requires in
  load order), `models.rb` (the top-level aliases), the tests, the project files (gemspec,
  Gemfile, Rakefile, Steepfile, `scripts/`, `sorbet/`, tool configurations) and the runtime
  template's options.
- `meta.rs`: the one accessor for Ruby settings, `ruby_target` (`targets.ruby`, overridden by
  `RubyMeta`, which also carries the release version, the README example and the contact email).

The plugin writes the generator's form; the gem's own formatters lay it out, as Stainless's
`rake format` does. `ferrule-gen generate --target ruby` writes the files and
`.ferrule/ruby-runtime.json`; `ferrule generate --target ruby` renders the runtime from it and runs
the formatters.

## The formatters

`tools/Gemfile.lock` pins the versions the oracles use (rubocop, syntax_tree and its RBS plugin, a
git gem). Install them once, with the Ruby that `mise.toml` pins:

```bash
mise run ruby:tools        # bundle install into .cache/ruby-tools (gitignored); network once
```

`tools/bin/stree` and `tools/bin/rubocop` run them from that bundle with no environment set up;
`ferrule generate --target ruby` and the tests use them by default. `$FERRULE_STREE` and
`$FERRULE_RUBOCOP` override them (words split on spaces, such as `bundle exec stree`). Without
either, the oracle comparisons and the golden test's formatting check skip with a note saying so.
To start over, delete `.cache/ruby-tools` and run the task again.

On macOS, building the native extensions (json, rbs, prism) can fail with `unknown architecture`
when the Command Line Tools' SDK is newer than their linker; point `SDKROOT` at a matching SDK
(such as Xcode's) for the install.

## Proof

`tests/oracle.rs` compares the formatted gem with real Stainless gems (Apache-2.0):

- onebusaway-ruby (`46a574dd`, the generation of the proven Python pin): 326 of 329 generated
  files byte-identical; the three others are Forge gap G18 (a path param's `example` is not in
  the model).
- imagekit-ruby (`65401086`): 349 of 432 identical; each of the 83 others is listed with its
  reason: model and Forge gaps (webhooks, `x-stainless-variantName`, union variant order,
  multi-value discriminator mappings, operation-level path param docs, G18 examples, schema
  titles), converter gap FER1004, imagekit's hand edits and older project files, and three
  unexplained rules (field order of `token`, OrHash on a recursive model, hard breaks).

`tests/golden.rs` pins a synthetic API (bodies, a subresource, a method without a response,
shared models, a map-of-objects converter constant) as `ferrule-gen` writes it, and checks that
the formatters accept it and settle after one pass. Both oracle gems pass `bundle exec rake test`
against a stand-in mock.

## Deliberate differences

- The `.keep` files' text names Ferrule, as for Python
  (`crates/python-plugin/src/emit/files/BRANDING.md`).

## Not done yet

- Project files: README, CONTRIBUTING, SECURITY, LICENSE, CI, release-please files, `bin/`,
  `Gemfile.lock` and `.stats.yml`; `scripts/` are written without their exec bits.
- Pagination, streaming and webhooks; the gaps listed under Proof.
- The 2025 Stainless generation (knock-ruby, stagehand-ruby).
