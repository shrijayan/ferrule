# Changelog

## [0.1.1](https://github.com/shrijayan/ferrule/compare/ferrule-runtime-v0.1.0...ferrule-runtime-v0.1.1) (2026-11-25)


### Features

* **runtime:** add the C# runtime template from imagekit-dotnet ([d71fa3c](https://github.com/shrijayan/ferrule/commit/d71fa3cad57ecee924a1235ea51122e6c8c68202))
* **runtime:** add the Go v1 runtime template ([2692076](https://github.com/shrijayan/ferrule/commit/269207641a256553033df30e12e949f09135d852))
* **runtime:** add the JVM runtime template for Java and Kotlin SDKs ([1ad3304](https://github.com/shrijayan/ferrule/commit/1ad3304954b4fdaa97af8fa63f652e15d9ee1cc2))
* **runtime:** add the Ruby runtime template ([40dfda1](https://github.com/shrijayan/ferrule/commit/40dfda16e219e70b342acffd0f54fd68ee38885f))
* **runtime:** add the Rust runtime template ([a4f557c](https://github.com/shrijayan/ferrule/commit/a4f557c91ceabecda93b96a2030277f9cf13183c))
* **runtime:** add the TypeScript node-layout runtime template ([81c43db](https://github.com/shrijayan/ferrule/commit/81c43dbedf6ca476c9318822ed52aa34083925e0))
* **runtime:** add the TypeScript runtime template ([b672c0d](https://github.com/shrijayan/ferrule/commit/b672c0d14d619c7b74b6669b92ba73e587e1d57e))
* **runtime:** derive page schemes from the configured pagination ([950084e](https://github.com/shrijayan/ferrule/commit/950084e9db1f204391052c13dcbe51b38cfb1664))
* **runtime:** render the PHP runtime from imagekit-php's ([c1cf585](https://github.com/shrijayan/ferrule/commit/c1cf585493ea309821abe139b5fca55e78fdda36))
* **runtime:** replace tokens in template paths and add the fits directive ([0781af2](https://github.com/shrijayan/ferrule/commit/0781af20b81ddcc82b70820599437f8c0cf3c613))
* **runtime:** template Stainless's Go v2 runtime from imagekit-go ([3a3751b](https://github.com/shrijayan/ferrule/commit/3a3751bcb960a6fb93cb2e250e897914032e3fb1))
* **rust-plugin:** multipart uploads and fixes found on real specs ([b526405](https://github.com/shrijayan/ferrule/commit/b526405f8b24d4095fac8b5b8dfb14f08e6c32c7))


### Bug Fixes

* **csharp-plugin:** send an API key in the query ([980ffcd](https://github.com/shrijayan/ferrule/commit/980ffcdb602a4b8368a2331dd31bd5f2fb77d93e))
* **migrate:** render pagination.py from the annotated spec's page schemes ([82b7345](https://github.com/shrijayan/ferrule/commit/82b7345f06187cf7b43053ea7974cbd8077b05e6))
* **runtime:** derive the cursor_id item field from the page scheme ([b916704](https://github.com/shrijayan/ferrule/commit/b916704a9055d6677bff5744498cc43dea7eb5f9))
* **runtime:** keep Rust SDK credentials on the base URL's origin ([f091678](https://github.com/shrijayan/ferrule/commit/f091678cce049e4b3edfe41026e3af0f36e88f19))
* **runtime:** realign the Go RequestConfig runs a long client setting widens ([0c21e3b](https://github.com/shrijayan/ferrule/commit/0c21e3bd8a335f8453c65cdbaddddaaed25938cd))
* **runtime:** refuse two templates that render to one path ([44617f2](https://github.com/shrijayan/ferrule/commit/44617f2b3c1726dca097c1c0b4583893f0ef998f))
* **runtime:** reject dot-segment path params and check every URL's origin ([700d1a3](https://github.com/shrijayan/ferrule/commit/700d1a3019ed10cf4bc937a7d5fa3630217a17e2))
* **runtime:** reject empty and null path params ([c27974c](https://github.com/shrijayan/ferrule/commit/c27974c32bcd055555c4d23e0ff966d3c8a393e4))
* **runtime:** reject typescript client and error names that clash with the runtime ([2b4b5bb](https://github.com/shrijayan/ferrule/commit/2b4b5bb855c9dd6e38138bcd2ffe09269037bb25))
* **runtime:** sort every rendered Go import group by path, as gofmt does ([511f252](https://github.com/shrijayan/ferrule/commit/511f25292df469fd7637a64c7e1090cbfa684a13))

## 0.1.0 (2026-10-26)


### Features

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

* **csharp-plugin:** send an API key in the query ([980ffcd](https://github.com/shrijayan/ferrule/commit/980ffcdb602a4b8368a2331dd31bd5f2fb77d93e))
* **runtime:** realign the Go RequestConfig runs a long client setting widens ([0c21e3b](https://github.com/shrijayan/ferrule/commit/0c21e3bd8a335f8453c65cdbaddddaaed25938cd))
* **runtime:** refuse two templates that render to one path ([44617f2](https://github.com/shrijayan/ferrule/commit/44617f2b3c1726dca097c1c0b4583893f0ef998f))
* **runtime:** reject typescript client and error names that clash with the runtime ([2b4b5bb](https://github.com/shrijayan/ferrule/commit/2b4b5bb855c9dd6e38138bcd2ffe09269037bb25))
* **runtime:** sort every rendered Go import group by path, as gofmt does ([511f252](https://github.com/shrijayan/ferrule/commit/511f25292df469fd7637a64c7e1090cbfa684a13))
