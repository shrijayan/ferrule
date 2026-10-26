# Changelog

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
* **runtime:** add opt-in testing fake server to the runtime template ([62da6a9](https://github.com/shrijayan/ferrule/commit/62da6a913d4316b5c9b55e84360350041e9f50b9))
* **runtime:** add the C# runtime template from imagekit-dotnet ([d71fa3c](https://github.com/shrijayan/ferrule/commit/d71fa3cad57ecee924a1235ea51122e6c8c68202))
* **runtime:** add the Go v1 runtime template ([2692076](https://github.com/shrijayan/ferrule/commit/269207641a256553033df30e12e949f09135d852))
* **runtime:** add the JVM runtime template for Java and Kotlin SDKs ([1ad3304](https://github.com/shrijayan/ferrule/commit/1ad3304954b4fdaa97af8fa63f652e15d9ee1cc2))
* **runtime:** add the Ruby runtime template ([40dfda1](https://github.com/shrijayan/ferrule/commit/40dfda16e219e70b342acffd0f54fd68ee38885f))
* **runtime:** add the TypeScript node-layout runtime template ([81c43db](https://github.com/shrijayan/ferrule/commit/81c43dbedf6ca476c9318822ed52aa34083925e0))
* **runtime:** add the TypeScript runtime template ([b672c0d](https://github.com/shrijayan/ferrule/commit/b672c0d14d619c7b74b6669b92ba73e587e1d57e))
* **runtime:** render the PHP runtime from imagekit-php's ([c1cf585](https://github.com/shrijayan/ferrule/commit/c1cf585493ea309821abe139b5fca55e78fdda36))
* **runtime:** replace tokens in template paths and add the fits directive ([0781af2](https://github.com/shrijayan/ferrule/commit/0781af20b81ddcc82b70820599437f8c0cf3c613))
* **runtime:** template Stainless's Go v2 runtime from imagekit-go ([3a3751b](https://github.com/shrijayan/ferrule/commit/3a3751bcb960a6fb93cb2e250e897914032e3fb1))

### Bug Fixes

* **migrate:** find hand edits the last Stainless commit carried ([72ebd06](https://github.com/shrijayan/ferrule/commit/72ebd06cf3e4f771ffb3c91e6f7730e911d86bc1))
* **migrate:** mark the generated scripts executable ([6d97681](https://github.com/shrijayan/ferrule/commit/6d976812ab942146efec86446aa220a8f37c4f31))
* **migrate:** pass the adopted runtime to convert so it can read the generator style ([95f44c1](https://github.com/shrijayan/ferrule/commit/95f44c1f1cf0c2e110f5a9a66f2d4fad522f81ce))
* **migrate:** refuse .git variants that file systems fold together ([81b070e](https://github.com/shrijayan/ferrule/commit/81b070e954e11f3575830b1f12c761bb73e82e87))
* **migrate:** refuse colons in merge tree paths ([d3876d2](https://github.com/shrijayan/ferrule/commit/d3876d24b3f67f8e4d9b8ed3addcd72295bd1127))
* **migrate:** refuse tree paths that escape the output in merge ([620e38e](https://github.com/shrijayan/ferrule/commit/620e38e45cd04d0cceff3dd1ab2a9677a2b4521c))
* **csharp-plugin:** send an API key in the query ([980ffcd](https://github.com/shrijayan/ferrule/commit/980ffcdb602a4b8368a2331dd31bd5f2fb77d93e))
* **runtime:** realign the Go RequestConfig runs a long client setting widens ([0c21e3b](https://github.com/shrijayan/ferrule/commit/0c21e3bd8a335f8453c65cdbaddddaaed25938cd))
* **runtime:** refuse two templates that render to one path ([44617f2](https://github.com/shrijayan/ferrule/commit/44617f2b3c1726dca097c1c0b4583893f0ef998f))
* **runtime:** reject typescript client and error names that clash with the runtime ([2b4b5bb](https://github.com/shrijayan/ferrule/commit/2b4b5bb855c9dd6e38138bcd2ffe09269037bb25))
* **runtime:** sort every rendered Go import group by path, as gofmt does ([511f252](https://github.com/shrijayan/ferrule/commit/511f25292df469fd7637a64c7e1090cbfa684a13))
