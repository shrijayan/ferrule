# ferrule-php-plugin

The PHP plugin: the neutral model of `ferrule-sdk-model` to a Composer package in Stainless's PHP
layout: one class per model, enum and union under `src/` (nested by namespace, `Files\File\AITag`),
and per resource a service, a raw service and their two contracts. It never imports Forge; the
model comes from `ferrule_sdk_model::forge_to_model` (`tests/boundary.rs`).

```text
annotated spec --sdk-model adapter--> model::Client --for_target("php")--> namer + tree + emitters
  --> generated files + runtime options --ferrule_runtime (runtime/php)--> package
  --> php-cs-fixer (the package's .php-cs-fixer.dist.php), as scripts/format runs it
```

- `naming.rs`: class, property, method and enum-case names with Stainless's initialisms and
  `x-ferrule-casings` (`URLEndpoints`, `fileID`, `aiTags`, `_240`, `Mixed_`).
- `types.rs`: the one type mapper (`PType`): native types of properties and parameters, the phpdoc
  of `@var`, `@param`, `@return` and the `@phpstan-type` shapes, the `#[Required]`/`#[Optional]`
  converters; and `Imports`, which decides per file whether a class is imported, named relative to
  an imported class, or fully qualified (php-cs-fixer imports and shortens those where it can).
- `tree.rs`: the declarations built from the model: field order, `allOf` bases flattened, inline
  objects, enums and unions nested, models on a reference cycle typed `mixed` in phpdoc, a method's
  params class (path parameters but the last, query, body; a union body's variants merged).
- `layout.rs`: the generator's line breaking (80 columns at 2 per level: a list in brackets is flat,
  hugged on one line of its own, or one item per line with trailing commas), which php-cs-fixer
  keeps while it re-indents to 4.
- `emit/`: models, services and contracts, the client and version, the tests, the project files
  and the runtime template's options.
- `meta.rs`: the one accessor for PHP settings, `php_target` (`targets.php`, overridden by
  `PhpMeta`, which also carries the release version, the example request and
  `disable_mock_tests`).

`ferrule-gen generate --target php` writes the files and `.ferrule/php-runtime.json`;
`ferrule generate --target php` runs php-cs-fixer on them and renders the runtime from it.

## The formatter

`tools/composer.json` pins php-cs-fixer 3.92.5, the version imagekit-php's `composer.lock` pins
(its dependencies at current releases, `tools/composer.lock`). Install it once, with the PHP that
`tools/env.sh` pins (8.3, as imagekit-php's CI and composer platform). PHP is not in `mise.toml`'s
tools, since mise builds it from source; the task installs it the first time:

```bash
mise run php:tools         # builds PHP 8.3.36 (needs autoconf, bison, re2c, libxml2, ... from
                           # Homebrew), then composer install into .cache/php-tools (gitignored)
```

`tools/bin/php-cs-fixer` runs it with no environment set up; `ferrule generate --target php` and
the tests use it by default, and `$FERRULE_PHP_CS_FIXER` overrides it (words split on spaces).
Without either, the oracle comparison and the golden test's formatting check skip with a note
saying so. To start over, delete `.cache/php-tools` and run the task again.

On macOS, building PHP can fail with `C compiler cannot create executables` when the Command Line
Tools' newest SDK is newer than their linker; point `SDKROOT` at a matching SDK for the install
(`SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX26.sdk mise run php:tools`).

## Proof

`tests/oracle.rs` compares the formatted package with imagekit-php (`9f9309d5`, Apache-2.0): 268
of the 336 generated files the oracle also has are byte-identical (with the 48 runtime files, 316
of 384), and the other 86 files the plugin writes are the oracle's under other names. Every
difference is listed with its reason:

- model gap: a union variant is named after its schema title (`GoogleCloudStorageGcs`, `Mixed_`,
  `RemovedotBgExtension`), which the model does not carry: 32 files, and the variant files under
  17 directories that the two name differently;
- Forge gap G18: parameter examples (the service tests' values): 10;
- model gap: the adapter lists a union's const variants last (Stainless keeps spec order): 9;
- unexplained: `token` kept third among the upload fields (as in imagekit-ruby), and one shape
  import's place in the files services: 6; plus imagekit's hand-edited upload host in the raw
  upload methods: 2;
- model gap: a model's module is the converter's (Python's), not the resource that declares it, so
  a model only another resource uses lands there (`Folder`: Assets, not Files): 5;
- Forge gap: webhook_unwrap methods and their event models (the webhooks service and 85 model
  files are not emitted; composer's `standard-webhooks`); Client.php also keeps imagekit's
  hand-edited `baseUrlOverridden`: 2;
- model gap: a schema title read as a variant's description: 2.

`tests/golden.rs` pins a synthetic API (bearer auth, integer path and query params, a date-time
query param, a map of objects, a union response, a subresource, a custom casing) as `ferrule-gen`
writes it, and checks that php-cs-fixer accepts it and settles after one pass.

`ferrule generate --target php` on the imagekit config, with the package's `composer install`,
passes the package's own tests (`vendor/bin/pest`, 87 tests) against a stand-in mock that answers
each operation with a minimal value of its response schema.

## Deliberate differences

None: the project files are imagekit-php's, which name no generator.

## Not done yet

- Project files: README, SECURITY, LICENSE, CI, release-please files, `bin/`, `composer.lock`
  and `.stats.yml`; `scripts/` are written without their exec bits; `scripts/test` is imagekit's
  form (mock tests disabled).
- Pagination (`SdkPage`), streaming and webhooks; the gaps under Proof.
- `disable_mock_tests` and the README example request are not in the model: `ferrule-gen` uses
  the defaults (`PhpMeta::default()`).
- Unverified with one oracle: the authentication headers other than HTTP basic (bearer and API
  key follow the other SDKs), integer enums, and the docblock widths of nested shapes.
