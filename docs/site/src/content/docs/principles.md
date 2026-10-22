---

title: Engineering principles
description: Rules the code is reviewed against.
---

**Stainless output is the spec**
When unsure, the oracle repos decide. Any deviation needs an ADR.

**Forge stays behind the adapter**
Only adapter/ imports Forge types. Emitters see our model only.

**DRY across sync and async**
One emitter pass with a mode flag. Type mapping lives in one function.

**Runtime written once**
The runtime lives in runtime/, tested once, copied into each SDK. SDKs never depend on Ferrule or Forge.

**Deterministic output**
Same inputs give byte-identical files. No timestamps, no LLM calls at generation time.

**Pure functions**
Convert and emit are pure. File and network access only in the CLI.

**Diagnostics with addresses**
Every error has a FER code, a pointer into the spec or config, and a suggested fix.

**Upstream first**
Anything generic (a Forge bug, a missing field) goes to Forge as an issue or PR, not a local workaround.
