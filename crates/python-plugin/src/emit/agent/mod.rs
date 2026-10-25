//! Agent context shipped inside every generated SDK (plan §5 item 4, ADR 0013): SKILL.md, AGENTS.md
//! and llms.txt under `src/<pkg>/_agent/` so the wheel carries them, locked to its version; with
//! project metadata also the repository copies (plan §10), one runnable recipe per example under
//! `examples/agent/` and a test that runs them against a mock. Pure: [`SdkClient`] in, path -> text
//! out. Not part of the Stainless output.

use super::common::{method_params, py_string, sort_fields, stainless_method_order, stream_switch};
use super::files::api_tests::{Uses, example_value};
use super::files_input::ProjectMeta;
use crate::model::{SdkClient, SdkClientOpt, SdkMethod, SdkResource};

/// One documented call: the code an agent copies, and whether a recipe runs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snippet {
    pub title: String,
    /// Recipe file stem, such as `stop_retrieve`.
    pub name: String,
    pub imports: Vec<String>,
    /// The statements after `client = ...`.
    pub body: Vec<String>,
    /// Plain request-response calls run as recipes; paginated and streaming ones are shown only.
    pub runnable: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentContext {
    pub pkg: String,
    pub distribution: String,
    pub version: String,
    pub client_class: String,
    pub env_prefix: String,
    /// The API key option, if the client reads one from the environment.
    pub auth: Option<SdkClientOpt>,
    pub repo: String,
    pub quickstart: Option<Snippet>,
    pub resources: Vec<Snippet>,
    pub pagination: Option<Snippet>,
    pub streaming: Option<Snippet>,
}

fn all_resources(resources: &[SdkResource]) -> Vec<&SdkResource> {
    resources
        .iter()
        .flat_map(|r| std::iter::once(r).chain(all_resources(&r.subresources)))
        .collect()
}

/// Keyword arguments for the required parameters, path parameters first.
fn call_args(method: &SdkMethod, uses: &mut Uses) -> Vec<String> {
    let params = method_params(method);
    let required = sort_fields(params.into_iter().filter(|p| p.required));
    method
        .path_params
        .iter()
        .chain(required)
        .map(|p| {
            let value = example_value(
                &p.ty,
                uses,
                "",
                p.example.as_ref(),
                &p.wire_name,
                p.minimum.or(p.maximum),
            );
            format!("{}={value}", p.py_name)
        })
        .collect()
}

struct Ctx<'a> {
    pkg: &'a str,
    client_class: &'a str,
}

fn program(
    ctx: &Ctx<'_>,
    resource: &SdkResource,
    method: &SdkMethod,
    body: impl FnOnce(&str) -> Vec<String>,
    extra: &[String],
) -> (Vec<String>, Vec<String>) {
    let mut uses = Uses::new();
    let mut args = call_args(method, &mut uses);
    args.extend(extra.iter().cloned());
    let call = format!(
        "client.{}.{}({})",
        resource.path.join("."),
        method.name,
        args.join(", ")
    );
    let mut imports = vec![format!("from {} import {}", ctx.pkg, ctx.client_class)];
    if !uses.is_empty() {
        imports.push(format!(
            "from {}._utils import {}",
            ctx.pkg,
            uses.iter().copied().collect::<Vec<_>>().join(", ")
        ));
    }
    (imports, body(&call))
}

/// A snippet as a complete program.
#[must_use]
pub fn snippet_code(client_class: &str, s: &Snippet) -> String {
    let mut lines: Vec<String> = s.imports.clone();
    lines.push(String::new());
    lines.push(format!("client = {client_class}()"));
    lines.push(String::new());
    lines.extend(s.body.iter().cloned());
    lines.push(String::new());
    lines.join("\n")
}

fn snippet_name(resource: &SdkResource, method: &SdkMethod) -> String {
    let mut parts: Vec<&str> = resource.path.iter().map(String::as_str).collect();
    parts.push(&method.name);
    parts.join("_")
}

fn plain(ctx: &Ctx<'_>, r: &SdkResource, m: &SdkMethod) -> Snippet {
    let variable = r.path.last().map_or("result", String::as_str);
    let (imports, body) = program(
        ctx,
        r,
        m,
        |call| vec![format!("{variable} = {call}"), format!("print({variable})")],
        &[],
    );
    Snippet {
        title: format!(
            "`client.{}.{}` ({} {})",
            r.path.join("."),
            m.name,
            m.http_method.as_str().to_uppercase(),
            m.path
        ),
        name: snippet_name(r, m),
        imports,
        body,
        runnable: m.pagination.is_none() && m.streaming.is_none(),
    }
}

/// The first method of a resource with no required parameters, else its first method.
fn example_method(r: &SdkResource) -> Option<&SdkMethod> {
    let methods = stainless_method_order(&r.methods);
    methods
        .iter()
        .copied()
        .find(|m| m.path_params.is_empty() && method_params(m).iter().all(|p| !p.required))
        .or_else(|| methods.first().copied())
}

#[must_use]
pub fn agent_context(client: &SdkClient, meta: Option<&ProjectMeta>) -> Option<AgentContext> {
    let s = client.settings.as_ref()?;
    let ctx = Ctx {
        pkg: &client.package.name,
        client_class: &s.class_name,
    };
    let resources = all_resources(&client.resources);
    let examples: Vec<Snippet> = resources
        .iter()
        .filter_map(|r| example_method(r).map(|m| plain(&ctx, r, m)))
        .collect();
    let methods: Vec<(&SdkResource, &SdkMethod)> = resources
        .iter()
        .flat_map(|&r| stainless_method_order(&r.methods).into_iter().map(move |m| (r, m)))
        .collect();
    let paged = methods.iter().find(|(_, m)| m.pagination.is_some());
    let streamed = methods.iter().find(|(_, m)| m.streaming.is_some());
    let pagination = paged.map(|&(r, m)| {
        let (imports, body) = program(
            &ctx,
            r,
            m,
            |call| {
                vec![
                    "# Iterating the page fetches the next pages as needed.".to_owned(),
                    format!("for item in {call}:"),
                    "    print(item)".to_owned(),
                ]
            },
            &[],
        );
        Snippet {
            title: "Pagination".to_owned(),
            name: format!("{}_pages", snippet_name(r, m)),
            imports,
            body,
            runnable: false,
        }
    });
    let streaming = streamed.map(|&(r, m)| {
        // The switch of a param-switched method, by its Python name (`stream_response`).
        let extra: Vec<String> = m
            .streaming
            .as_ref()
            .and_then(|st| st.param_discriminator.as_ref())
            .map(|d| {
                let name = stream_switch(m).map_or(d.as_str(), |q| q.py_name.as_str());
                vec![format!("{name}=True")]
            })
            .unwrap_or_default();
        let (imports, body) = program(
            &ctx,
            r,
            m,
            |call| {
                vec![
                    format!("stream = {call}"),
                    "for event in stream:".to_owned(),
                    "    print(event)".to_owned(),
                ]
            },
            &extra,
        );
        Snippet {
            title: "Streaming".to_owned(),
            name: format!("{}_stream", snippet_name(r, m)),
            imports,
            body,
            runnable: false,
        }
    });
    Some(AgentContext {
        pkg: client.package.name.clone(),
        distribution: meta
            .and_then(|m| m.distribution_name.clone())
            .unwrap_or_else(|| client.package.name.clone()),
        version: meta.map_or_else(|| "0.0.1".to_owned(), |m| m.version.clone()),
        client_class: s.class_name.clone(),
        env_prefix: s.env_prefix.clone(),
        auth: s
            .opts
            .iter()
            .find(|o| o.auth.is_some() && o.read_env.is_some())
            .cloned(),
        repo: client.package.repo.clone(),
        quickstart: examples
            .iter()
            .find(|e| e.runnable)
            .or_else(|| examples.first())
            .cloned(),
        resources: examples,
        pagination,
        streaming,
    })
}

fn fence(code: &str) -> String {
    format!("```python\n{code}```")
}

/// The API key option and the variable it reads, when there is one.
fn auth_env(c: &AgentContext) -> Option<(&SdkClientOpt, &str)> {
    c.auth.as_ref().and_then(|a| a.read_env.as_deref().map(|env| (a, env)))
}

#[must_use]
pub fn emit_skill_md(c: &AgentContext) -> String {
    let mut lines: Vec<String> = vec![
        "---".to_owned(),
        format!("name: {}", c.pkg),
        format!(
            "description: Use the {} Python SDK (version {}) to call the {} API: client setup, authentication, one example per resource, errors and retries.",
            c.distribution, c.version, c.client_class
        ),
        "---".to_owned(),
        String::new(),
        format!("# {} {}", c.distribution, c.version),
        String::new(),
        format!(
            "This skill ships inside the `{}` package, so it matches the installed version. The full method and type reference is `api.md` in the repository (https://github.com/{}).",
            c.distribution, c.repo
        ),
        String::new(),
        "## Install".to_owned(),
        String::new(),
        "```sh".to_owned(),
        format!("pip install {}=={}", c.distribution, c.version),
        "```".to_owned(),
        String::new(),
        "## Authenticate and create a client".to_owned(),
        String::new(),
    ];
    if let Some((auth, env)) = auth_env(c) {
        lines.push(format!(
            "Set `{env}`; the client reads it when `{}` is not passed. Never hard-code the key.",
            auth.name
        ));
        lines.push(String::new());
    }
    let mut setup = vec![
        "import os".to_owned(),
        String::new(),
        format!("from {} import {}", c.pkg, c.client_class),
        String::new(),
    ];
    setup.push(format!("client = {}(", c.client_class));
    if let Some((auth, env)) = auth_env(c) {
        setup.push(format!(
            "    {}=os.environ.get({}),  # the default",
            auth.name,
            py_string(env)
        ));
    }
    setup.push("    max_retries=2,  # the default; connection errors, 408, 409, 429 and 5xx are retried".to_owned());
    setup.push(")".to_owned());
    setup.push(String::new());
    lines.push(fence(&setup.join("\n")));
    lines.push(String::new());
    lines.push(format!(
        "Use `Async{}` the same way, with `await` on every call. `{}_BASE_URL` overrides the base URL.",
        c.client_class, c.env_prefix
    ));
    lines.push(String::new());
    if !c.resources.is_empty() {
        lines.extend([
            "## Examples".to_owned(),
            String::new(),
            "One call per resource; the arguments are placeholders.".to_owned(),
            String::new(),
        ]);
        for s in &c.resources {
            lines.extend([
                format!("### {}", s.title),
                String::new(),
                fence(&snippet_code(&c.client_class, s)),
                String::new(),
            ]);
        }
    }
    for s in [&c.pagination, &c.streaming].into_iter().flatten() {
        lines.extend([
            format!("## {}", s.title),
            String::new(),
            fence(&snippet_code(&c.client_class, s)),
            String::new(),
        ]);
    }
    lines.extend([
        "## Errors".to_owned(),
        String::new(),
        fence(&errors_example(c)),
        String::new(),
        "Every method also has `.with_raw_response` (headers and the raw response) and `.with_streaming_response`."
            .to_owned(),
        String::new(),
    ]);
    lines.join("\n")
}

/// The program of the SKILL.md Errors section: the quickstart call inside `try:`.
fn errors_example(c: &AgentContext) -> String {
    let mut errors = vec![format!("import {}", c.pkg)];
    match &c.quickstart {
        Some(q) => errors.extend(q.imports.iter().cloned()),
        None => errors.push(format!("from {} import {}", c.pkg, c.client_class)),
    }
    errors.extend([
        String::new(),
        format!("client = {}()", c.client_class),
        String::new(),
        "try:".to_owned(),
    ]);
    match &c.quickstart {
        None => errors.push("    pass".to_owned()),
        Some(q) => errors.extend(q.body.join("\n").split('\n').map(|l| format!("    {l}"))),
    }
    errors.extend([
        format!("except {}.APIConnectionError as e:", c.pkg),
        "    print(\"The server could not be reached:\", e.__cause__)".to_owned(),
        format!("except {}.RateLimitError:", c.pkg),
        "    print(\"Rate limited (429); back off and retry.\")".to_owned(),
        format!("except {}.APIStatusError as e:", c.pkg),
        "    print(\"Non-2xx response:\", e.status_code, e.response)".to_owned(),
        String::new(),
    ]);
    errors.join("\n")
}

#[must_use]
pub fn emit_agents_md(c: &AgentContext) -> String {
    let env = c
        .auth
        .as_ref()
        .and_then(|a| a.read_env.as_ref())
        .map_or_else(String::new, |e| format!(" (`{e}`)"));
    [
        format!("# Using {} {}", c.distribution, c.version),
        String::new(),
        format!("Instructions for coding agents writing code that uses the `{}` package.", c.pkg),
        String::new(),
        format!(
            "- Read `SKILL.md` next to this file first: install, authentication{env}, one example per resource and error handling, for this exact version."
        ),
        format!(
            "- Look up method signatures and types in `api.md` (https://github.com/{}/blob/main/api.md); every request and response type is importable from `{}.types`.",
            c.repo, c.pkg
        ),
        format!(
            "- Construct one `{0}` (or `Async{0}`) and reuse it; do not pass secrets in code.",
            c.client_class
        ),
        format!(
            "- Catch `{}.APIError` subclasses rather than `httpx` errors; retries and timeouts are already built in.",
            c.pkg
        ),
        "- Do not edit generated files of the SDK; they are regenerated from the API spec.".to_owned(),
        String::new(),
    ]
    .join("\n")
}

#[must_use]
pub fn emit_llms_txt(c: &AgentContext) -> String {
    let base = format!("https://github.com/{}/blob/main", c.repo);
    [
        format!("# {}", c.distribution),
        String::new(),
        format!(
            "> Python SDK for the {} API, version {}. Install with `pip install {}` and import `{}`.",
            c.client_class, c.version, c.distribution, c.pkg
        ),
        String::new(),
        "## Docs".to_owned(),
        String::new(),
        format!("- [README]({base}/README.md): installation, usage, async client, retries, timeouts"),
        format!("- [api.md]({base}/api.md): every resource, method and type"),
        format!(
            "- [SKILL.md]({base}/src/{}/_agent/SKILL.md): agent skill with tested examples",
            c.pkg
        ),
        String::new(),
    ]
    .join("\n")
}

const RECIPE_HEADER: &str = "# Agent recipe generated by Ferrule from SKILL.md; tests/test_agent_recipes.py runs it.";

#[must_use]
pub fn emit_recipe_test(c: &AgentContext) -> String {
    let mut lines: Vec<String> = [
        "# Generated by Ferrule: runs every examples/agent/ recipe against a mock server.",
        "",
        "from __future__ import annotations",
        "",
        "import os",
        "import runpy",
        "from pathlib import Path",
        "",
        "import httpx",
        "import pytest",
        "from respx import MockRouter",
        "",
        "base_url = os.environ.get(\"TEST_API_BASE_URL\", \"http://127.0.0.1:4010\")",
        "RECIPES = sorted((Path(__file__).parent.parent / \"examples\" / \"agent\").glob(\"*.py\"))",
        "",
        "",
        "@pytest.mark.parametrize(\"recipe\", RECIPES, ids=[p.stem for p in RECIPES])",
        "@pytest.mark.respx(base_url=base_url)",
        "def test_recipe_runs(recipe: Path, respx_mock: MockRouter, monkeypatch: pytest.MonkeyPatch) -> None:",
        "    respx_mock.route().mock(return_value=httpx.Response(200, json={}))",
    ]
    .iter()
    .map(|&l| l.to_owned())
    .collect();
    if let Some(env) = c.auth.as_ref().and_then(|a| a.read_env.as_ref()) {
        lines.push(format!("    monkeypatch.setenv({}, \"My API Key\")", py_string(env)));
    }
    lines.push(format!(
        "    monkeypatch.setenv({}, base_url)",
        py_string(&format!("{}_BASE_URL", c.env_prefix))
    ));
    lines.push("    runpy.run_path(str(recipe), run_name=\"__main__\")".to_owned());
    lines.push(String::new());
    lines.join("\n")
}

/// Every agent-context file, in the order the TypeScript plugin adds them; the repository
/// copies and recipes need project metadata.
#[must_use]
pub fn emit_agent_files(client: &SdkClient, meta: Option<&ProjectMeta>) -> Vec<(String, String)> {
    let mut files = Vec::new();
    let Some(c) = agent_context(client, meta) else {
        return files;
    };
    let skill = emit_skill_md(&c);
    let agents = emit_agents_md(&c);
    let llms = emit_llms_txt(&c);
    files.push((format!("src/{}/_agent/SKILL.md", c.pkg), skill.clone()));
    files.push((format!("src/{}/_agent/AGENTS.md", c.pkg), agents.clone()));
    files.push((format!("src/{}/_agent/llms.txt", c.pkg), llms.clone()));
    if meta.is_none() {
        return files;
    }
    files.push((format!("skills/{}/SKILL.md", c.pkg), skill));
    files.push(("AGENTS.md".to_owned(), agents));
    files.push(("llms.txt".to_owned(), llms));
    let recipes: Vec<&Snippet> = c.resources.iter().filter(|s| s.runnable).collect();
    for s in &recipes {
        files.push((
            format!("examples/agent/{}.py", s.name),
            format!("{RECIPE_HEADER}\n\n{}", snippet_code(&c.client_class, s)),
        ));
    }
    if !recipes.is_empty() {
        files.push(("tests/test_agent_recipes.py".to_owned(), emit_recipe_test(&c)));
    }
    files
}
