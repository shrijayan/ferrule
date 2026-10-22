## Reality check: your premise is out of date

An "open-source Stainless alternative" is no longer an empty slot. Speakeasy open-sourced its 7-language generator under AGPL with Google on Sep 17 ([Google](https://developers.googleblog.com/en/why-client-sdk-generation-belongs-in-the-open/)). Cloudflare shipped Forge under Apache-2.0 ([blog](https://blog.cloudflare.com/forge-open-source-generation-pipeline/)); it has about 1.1k stars and no Python target yet. Fern's generator is still Apache-2.0 under Postman. Scalar (closed source) and stainful already import `stainless.yml` ([Scalar](https://scalar.com/resources/stainless-wind-down)). stainful's last commit was May 22 ([repo](https://github.com/stainlu/stainful)). Being open source and reading `stainless.yml` are now the minimum. Ferrule has to win on things those players structurally won't do.

## Top 7 differentiators (impact / effort)

**1. Proven surface fidelity: "your users won't notice you left." (High / Medium)** Stainless repos ship their own test oracle. `api.md` lists the full public surface ([example](https://github.com/openai/openai-python/blob/main/api.md)), and `.stats.yml` records the spec URL, spec hash and config hash ([example](https://github.com/perplexityai/perplexity-py/blob/main/.stats.yml)). `ferrule verify` would:
- regenerate from that exact spec;
- run griffe's breaking-change detector ([griffe](https://pypi.org/project/griffe/)) against the last Stainless release, covering signatures, keyword-only parameters, defaults, exception hierarchy and import paths;
- replay recorded HTTP sessions to check pagination, retries and timeouts;
- print `0 breaking / N additive`.

Competitors sell their own house style. Committing to a former rival's design as the spec is something they won't do.

**2. Output that's truly yours. (High / Low)** Speakeasy's [LICENSING.md](https://github.com/speakeasy-api/openapi-generation/blob/main/LICENSING.md) says that without a commercial authorization, the Speakeasy-authored code inside generated SDKs is AGPL-3.0-only. That's a poison pill for a public SDK embedded in customer apps. Ferrule's answer: an explicit output exception (emitted code carries no license obligations), no license token, and no runtime calls home. Speakeasy can't copy this without giving up its revenue.

**3. First hour: `uvx ferrule adopt .` (High / Low)**
- **Minute 1:** read `.stats.yml` and download the spec from Stainless's bucket (still live today; it could vanish). Check it against the recorded hash and commit it to the repo.
- **Minute 5:** import `stainless.yml`.
- **Minute 15:** regenerate and produce the fidelity report.
- **Minute 30:** diff hand edits against the last generated commit and save them as a named patch ledger. OpenAI's new in-house setup tracks custom patch lines (`max_custom_patch_lines: 10000`, [file](https://github.com/openai/openai-python/blob/main/.castiron-ratchet.json)), so patches are the real lock-in.
- **Minute 45:** open a PR that adds the GitHub Action.

Spec rescue is useful even to teams that never switch, which makes it the top of the funnel.

**4. Acquisition-proof by construction. (High / Low)** In 12 months, Stainless went to Anthropic, Fern and liblab went to Postman ([Fern](https://buildwithfern.com/post/postman-acquires-fern), [liblab](https://www.businesswire.com/news/home/20251114198999/en)), and OpenAI announced it is buying Astral ([Simon Willison](https://simonwillison.net/2026/Mar/19/openai-acquiring-astral/)). Ferrule's structure:
- **DCO, never a CLA.** Speakeasy's [CLA](https://github.com/speakeasy-api/openapi-generation/blob/main/CLA.md) explicitly grants it the right to relicense contributions. Without a CLA, relicensing Ferrule would need every contributor's consent.
- **A neutral home for the name and domain.** Commonhaus lets projects keep their own governance ([TNS](https://thenewstack.io/commonhaus-foundation-launches-at-critical-time-for-oss/)). The OpenAPI Initiative hosts specs, not tools.

**5. Reproducible, attestable, offline. (Medium-High / Low)**
- Byte-identical output for the same inputs, recorded in `ferrule.lock` (spec, config and generator digests).
- SLSA provenance via [attest-build-provenance](https://docs.github.com/actions/security-guides/using-artifact-attestations-to-establish-provenance-for-builds) and [PyPI attestations](https://docs.pypi.org/attestations/). Note that private repos need GitHub Enterprise Cloud for attestations.
- An SBOM, zero telemetry and an `--offline` flag.

The real enterprise win is skipping procurement: nothing leaves the laptop, so there's no vendor security review. A hosted vendor can't offer that.

**6. Semver-aware regeneration bot. (Medium / Medium)** Restores exactly what Stainless customers lost: regeneration whenever the API changes ([Scalar](https://scalar.com/resources/stainless-wind-down)). It works like Renovate: watch the spec hash, open a PR, and put the griffe breakage set in the PR body with an automatic major/minor/patch label.

**7. Public fidelity board. (Medium / Medium)** GitHub code search finds 634 public repos with Stainless `.stats.yml` files. Every night, regenerate the Python ones and publish their scores. Add messy specs from APIs.guru, plus a "bring your spec" badge. Ship the Python emitter as a Forge transformer too, so Ferrule reaches users through Cloudflare's distribution instead of fighting it.

## Economics

Stainless charged $79–$499 a month per generator, where each SDK counted ([pricing](https://www.stainless.com/pricing)). Speakeasy reportedly charges around $600–720 a month per language ([Fern](https://buildwithfern.com/post/speakeasy-reviews-pricing-alternatives)). At zero marginal cost, new kinds of SDKs become viable:
- internal-service SDKs;
- per-version and preview SDKs;
- SDKs for small APIs.

The buyer also changes, from a DevRel budget line to a platform engineer who needs no purchase order.

## Open vs. paid

**Open forever (Apache-2.0, plus the output exception):** everything that runs on your machine. That covers the generator, every language target, the importer, verifier, Action, bot and corpus. No features gated behind enterprise tiers.

**Paid (sold by a separate company or by maintainers, as Biome does through [contributors](https://biomejs.dev/enterprise)):**
- fixed-fee "Stainless exit" migrations with a signed fidelity report;
- LTS branches with backported fixes;
- a security-response SLA;
- indemnity.

**Never:** hosted generation, license tokens, or open-core features.

Lessons from the reference companies:
- **Astral** shows the free-tools-plus-paid-hosting model works ([pyx](https://astral.sh/blog/introducing-pyx)), but also that it makes the company acquirable.
- **Grafana** moved from Apache-2.0 to AGPL in 2021 ([blog](https://grafana.com/blog/2021/04/20/grafana-loki-tempo-relicensing-to-agplv3)). Ferrule should make that move impossible by design.
- **Sentry's FSL** isn't OSI open source. Don't copy it.

## First 100 adopters

1. Pull the 634-repo list and filter to Python. Skip OpenAI (in-house "castiron"), Anthropic and Cloudflare (Forge).
2. Target the long tail, the 4–50 endpoint startups like Cerebras or Perplexity. AGPL rules out Speakeasy's free path for them, and its commercial license costs too much.
3. Privately run `adopt` on the top 50. Send each maintainer their report, and open a PR only when breaking changes are zero.
4. Launch on Hacker News with data: "We regenerated N Stainless SDKs; here's every breaking change."
5. Ship spec rescue as a standalone, free one-liner.
6. Measure success as 100 repos with a committed `ferrule.lock`.

## Pitch

**"Leave Stainless without your users noticing: Ferrule regenerates your SDK locally and proves no public API changed. Apache-2.0, no CLA, and nobody to acquire."**
