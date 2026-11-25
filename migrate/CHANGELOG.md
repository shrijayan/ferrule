# Changelog

## [0.1.1](https://github.com/shrijayan/ferrule/compare/ferrule-v0.1.0...ferrule-v0.1.1) (2026-11-25)


### Features

* **migrate:** add ferrule regenerate, merging a new generation into the branch ([eee42b0](https://github.com/shrijayan/ferrule/commit/eee42b005ce70dd0eb2e6d7e9f44134f6641533a))
* **migrate:** default the Ruby formatters to the repository's bundle ([db5a1fe](https://github.com/shrijayan/ferrule/commit/db5a1fed521b488974225b82756f5fccd4632eb7))
* **migrate:** generate a C# SDK with ferrule generate --target csharp ([c69d08b](https://github.com/shrijayan/ferrule/commit/c69d08be3949f31735d38f6444f186b8604b8256))
* **migrate:** generate a Ruby gem with ferrule generate --target ruby ([b77f0e0](https://github.com/shrijayan/ferrule/commit/b77f0e044da85eed2e5d78d19048883256302b7e))
* **migrate:** generate Java and Kotlin SDKs with ferrule generate --target ([09f6e32](https://github.com/shrijayan/ferrule/commit/09f6e32cd37940fd9e3c8b8eb62a4b1f9320f672))
* **migrate:** generate PHP with ferrule generate --target php ([8e485ea](https://github.com/shrijayan/ferrule/commit/8e485ea78ac6a90413fddce6579bedaedbd85d9d))
* **migrate:** render the Go runtime in ferrule generate --target go ([b21147b](https://github.com/shrijayan/ferrule/commit/b21147b37f0279267a8c670b0816f489582f44f6))


### Bug Fixes

* **migrate:** adopt an SDK with no Stainless history from HEAD when its spec is given ([c7e5eef](https://github.com/shrijayan/ferrule/commit/c7e5eef9f7b6ccc5236a4151f1aded18b0901474))
* **migrate:** commit and lock the runtime adopt rendered ([56422a4](https://github.com/shrijayan/ferrule/commit/56422a43cc47518dcc3967016ca0f787a5e75786))
* **migrate:** find hand edits the last Stainless commit carried ([72ebd06](https://github.com/shrijayan/ferrule/commit/72ebd06cf3e4f771ffb3c91e6f7730e911d86bc1))
* **migrate:** keep adopt's pristine generation on the generated branch ([55ca8b2](https://github.com/shrijayan/ferrule/commit/55ca8b20c5f26c3f7cdaca84dec117ce7ef34337))
* **migrate:** keep file modes when regenerate commits a merge ([fac7a8e](https://github.com/shrijayan/ferrule/commit/fac7a8e2869fd41650869898a522dc0963634d9c))
* **migrate:** list runtime spellings of the same API apart from breaking changes ([f745fe3](https://github.com/shrijayan/ferrule/commit/f745fe3daa610716a44ca7fa84cd01c89ee68e2e))
* **migrate:** mark the generated scripts executable ([6d97681](https://github.com/shrijayan/ferrule/commit/6d976812ab942146efec86446aa220a8f37c4f31))
* **migrate:** merge a regeneration from the last generation the branch took ([dd98c8a](https://github.com/shrijayan/ferrule/commit/dd98c8a295fa944ca4d025042e2f4996147bc1cb))
* **migrate:** normalise runtime spellings of a class that moved module ([10cc1a7](https://github.com/shrijayan/ferrule/commit/10cc1a75f0e69cdf932e16ffd2af4debf731a666))
* **migrate:** pass the adopted runtime to convert so it can read the generator style ([95f44c1](https://github.com/shrijayan/ferrule/commit/95f44c1f1cf0c2e110f5a9a66f2d4fad522f81ce))
* **migrate:** read keep_files past comment and blank lines ([92d2efc](https://github.com/shrijayan/ferrule/commit/92d2efce3f11fa75db1a32dc1eef7a7efd314b99))
* **migrate:** read the configured retries from their nested object ([29554cb](https://github.com/shrijayan/ferrule/commit/29554cb1de2a3a63fb1df1e8ea4d4a2007ce7489))
* **migrate:** render Ferrule's runtime when the adopted SDK's runtime is not Stainless-style ([001c900](https://github.com/shrijayan/ferrule/commit/001c900340bbea264f4eafcbe4c340ec3edb534a))
* **migrate:** render pagination.py from the annotated spec's page schemes ([82b7345](https://github.com/shrijayan/ferrule/commit/82b7345f06187cf7b43053ea7974cbd8077b05e6))
* **migrate:** render the runtime with the configured timeout and retries ([b4aac14](https://github.com/shrijayan/ferrule/commit/b4aac14845960c9d0188551d9597d6d77c457e76))
* **migrate:** update the agent recipes Ferrule generates under examples/ ([de8cc49](https://github.com/shrijayan/ferrule/commit/de8cc4966d2fb630a438e75bcb420e9a36d90f49))
* **migrate:** write an sdk.yml that runs the merging action ([df2a964](https://github.com/shrijayan/ferrule/commit/df2a9646c64a27c24231f0e958a3cfb03c7b5efa))

## 0.1.0 (2026-10-26)


### Features

* **migrate:** add ferrule adopt and wire rescue/adopt into the CLI ([2cd76e2](https://github.com/shrijayan/ferrule/commit/2cd76e29f1a6e142797c81132e46fbb113616cfe))
* **migrate:** add ferrule merge and regenerate_branch ([f6882a5](https://github.com/shrijayan/ferrule/commit/f6882a5e576af9a1d99b0a3ad12d44d46b71db63))
* **migrate:** add spec rescue and ferrule.lock ([7e9073d](https://github.com/shrijayan/ferrule/commit/7e9073d0fdbe0394f97e69b95dc7ec083db30538))
* **migrate:** default the Ruby formatters to the repository's bundle ([db5a1fe](https://github.com/shrijayan/ferrule/commit/db5a1fed521b488974225b82756f5fccd4632eb7))
* **migrate:** generate a C# SDK with ferrule generate --target csharp ([c69d08b](https://github.com/shrijayan/ferrule/commit/c69d08be3949f31735d38f6444f186b8604b8256))
* **migrate:** generate a Ruby gem with ferrule generate --target ruby ([b77f0e0](https://github.com/shrijayan/ferrule/commit/b77f0e044da85eed2e5d78d19048883256302b7e))
* **migrate:** generate Java and Kotlin SDKs with ferrule generate --target ([09f6e32](https://github.com/shrijayan/ferrule/commit/09f6e32cd37940fd9e3c8b8eb62a4b1f9320f672))
* **migrate:** generate PHP with ferrule generate --target php ([8e485ea](https://github.com/shrijayan/ferrule/commit/8e485ea78ac6a90413fddce6579bedaedbd85d9d))
* **migrate:** pure three-way tree merge with libcst fallback ([6885f7a](https://github.com/shrijayan/ferrule/commit/6885f7ac230faa767de7d9c8d990d864b5933aed))
* **migrate:** render the Go runtime in ferrule generate --target go ([b21147b](https://github.com/shrijayan/ferrule/commit/b21147b37f0279267a8c670b0816f489582f44f6))
* **migrate:** run ferrule-gen instead of node, forge and plugin.wasm ([e53a119](https://github.com/shrijayan/ferrule/commit/e53a11982795ebc3d7186ffeb30405aa0d1b9999))


### Bug Fixes

* **migrate:** find hand edits the last Stainless commit carried ([72ebd06](https://github.com/shrijayan/ferrule/commit/72ebd06cf3e4f771ffb3c91e6f7730e911d86bc1))
* **migrate:** mark the generated scripts executable ([6d97681](https://github.com/shrijayan/ferrule/commit/6d976812ab942146efec86446aa220a8f37c4f31))
* **migrate:** pass the adopted runtime to convert so it can read the generator style ([95f44c1](https://github.com/shrijayan/ferrule/commit/95f44c1f1cf0c2e110f5a9a66f2d4fad522f81ce))
* **migrate:** refuse .git variants that file systems fold together ([81b070e](https://github.com/shrijayan/ferrule/commit/81b070e954e11f3575830b1f12c761bb73e82e87))
* **migrate:** refuse colons in merge tree paths ([d3876d2](https://github.com/shrijayan/ferrule/commit/d3876d24b3f67f8e4d9b8ed3addcd72295bd1127))
* **migrate:** refuse tree paths that escape the output in merge ([620e38e](https://github.com/shrijayan/ferrule/commit/620e38e45cd04d0cceff3dd1ab2a9677a2b4521c))
