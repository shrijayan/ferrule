//! `src/<Ns>.Tests/`: `TestBase.cs` (the client the tests share) and one test class per service.

use std::fmt::Write as _;

use crate::emit::client::opts;
use crate::emit::example::Examples;
use crate::emit::model::sort_usings;
use crate::emit::{Gen, Meth, Svc, template};
use crate::tree::{Decl, Kind, Loc};

/// `TestBase.cs`: the client with every required option set to a placeholder.
#[must_use]
pub fn test_base(g: &Gen<'_>) -> (String, String) {
    let t = g.target;
    let mut sets = String::new();
    // The tests set the string options a request needs: the required ones and the credentials.
    for o in opts(g)
        .into_iter()
        .filter(|o| o.ty.starts_with("string") && (!o.nullable || o.auth))
    {
        let title: Vec<String> = crate::naming::split_words(&o.name)
            .iter()
            .map(|w| {
                let lower = w.to_ascii_lowercase();
                if matches!(lower.as_str(), "id" | "api" | "url" | "ai" | "sdk") {
                    lower.to_ascii_uppercase()
                } else {
                    crate::naming::capitalize(&lower)
                }
            })
            .collect();
        let _ = writeln!(sets, "            {} = \"My {}\",", o.name, title.join(" "));
    }
    let text = template(include_str!("test_base.cs.txt"))
        .replace("__NS__", &t.ns)
        .replace("__CLIENT__", &t.client)
        .replace("__OPTIONS__", &sets);
    (format!("src/{}.Tests/TestBase.cs", t.ns), text)
}

/// The value a test passes for a path param: its wire name, or 0 for an integer.
fn path_value(m: &Meth<'_>, i: usize) -> String {
    if m.path_params[i].ty == "long" {
        "0".to_owned()
    } else {
        format!("\"{}\"", m.method.path_params[i].field.wire_name)
    }
}

/// `this.client.Files.Versions`.
fn chain(g: &Gen<'_>, s: &Svc<'_>) -> String {
    let mut parts = Vec::new();
    let mut keys: Vec<String> = s.resource.path.iter().map(|n| n.written.clone()).collect();
    while !keys.is_empty() {
        if let Some(svc) = g
            .services
            .iter()
            .find(|x| x.resource.path.iter().map(|n| n.written.clone()).collect::<Vec<_>>() == keys)
        {
            parts.push(svc.prop.clone());
        }
        keys.pop();
    }
    parts.reverse();
    format!("this.client.{}", parts.join("."))
}

/// `<Name>ServiceTest.cs`.
#[must_use]
pub fn service_test(g: &Gen<'_>, s: &Svc<'_>) -> Option<(String, String)> {
    if s.methods.is_empty() {
        return None;
    }
    let t = g.target;
    let fact = if t.disable_mock_tests {
        "[Fact(Skip = \"Mock server tests are disabled\")]"
    } else {
        "[Fact]"
    };
    let mut ns = format!("{}.Tests.Services", t.ns);
    for seg in &s.service_ns {
        ns.push('.');
        ns.push_str(seg);
    }
    let mut body = String::new();
    let mut examples = Examples::new(g);
    for (i, m) in s.methods.iter().enumerate() {
        if i > 0 {
            body.push('\n');
        }
        let mut args = Vec::new();
        let mut inits = Vec::new();
        if let Some(last) = m.path_params.len().checked_sub(1) {
            args.push(path_value(m, last));
            for (j, p) in m.path_params.iter().enumerate().take(last) {
                inits.push(format!("{} = {}", p.prop, path_value(m, j)));
            }
        }
        let si = g.services.iter().position(|x| std::ptr::eq(x, s)).unwrap_or_default();
        let params = g.files.files.iter().find(|f| f.kind == Kind::Params(si, i));
        if let Some(file) = params
            && let Some(Decl::Object(o)) = file.decls.first()
        {
            for p in o
                .props
                .iter()
                .filter(|p| p.required && p.constant.is_none() && p.loc != Loc::Path)
            {
                if let Some(v) = examples.value(&p.ty, &p.wire, file, false, 0) {
                    inits.push(format!("{} = {v}", p.name));
                }
            }
        }
        args.push(if inits.is_empty() {
            "new()".to_owned()
        } else {
            format!("new() {{ {} }}", inits.join(", "))
        });
        args.push("TestContext.Current.CancellationToken".to_owned());
        let call = format!("{}.{}(\n{}\n)", chain(g, s), m.name, args.join(",\n"));
        let _ = write!(body, "    {fact}\n    public async Task {}_Works()\n    {{\n", m.name);
        match &m.response {
            Some(r) => {
                let _ = writeln!(body, "        var {} = await {call};", r.var);
                if r.list {
                    let _ = writeln!(
                        body,
                        "        foreach (var item in {})\n        {{\n            item.Validate();\n        }}",
                        r.var
                    );
                } else {
                    let _ = writeln!(body, "        {}.Validate();", r.var);
                }
            }
            None => {
                let _ = writeln!(body, "        await {call};");
            }
        }
        body.push_str("    }\n");
    }
    let text = format!(
        "{}\nnamespace {ns};\n\npublic class {}ServiceTest : TestBase\n{{\n{body}}}\n",
        sort_usings(
            &std::iter::once("System.Threading.Tasks".to_owned())
                .chain(examples.usings)
                .collect::<Vec<_>>()
        ),
        s.prefix
    );
    let mut path = format!("src/{}.Tests/Services/", t.ns);
    for seg in &s.service_ns {
        path.push_str(seg);
        path.push('/');
    }
    let _ = write!(path, "{}ServiceTest.cs", s.prefix);
    Some((path, text))
}
