---

name: add-adr
description: Write a new Architecture Decision Record in docs/adr using the MADR template and register it in the index. Use for any decision or any deviation from Stainless output.
---

# Add an ADR

1. Pick the next number from `docs/adr/`. Filename `docs/adr/NNNN-kebab-title.md`.
2. Copy `docs/adr/template.md`; fill status (`Proposed` until a maintainer accepts), date (YYYY-MM-DD), context, options, decision and consequences.
3. If it supersedes an ADR, set the old one's status to `Superseded by NNNN` and link both ways.
4. Add a row to `docs/adr/README.md` and to `site/src/content/docs/docs/adr/index.md`.
5. Commit alone, signed off: `docs(adr): NNNN <title>`.
