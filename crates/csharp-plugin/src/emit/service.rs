//! `Services/**`: per resource, `I<Name>Service.cs` (the interface and its raw-response view) and
//! `<Name>Service.cs` (both implementations).

use std::fmt::Write as _;

use crate::doc;
use crate::emit::model::sort_usings;
use crate::emit::{Gen, Meth, PathParam, Svc};

const CT: &str = "CancellationToken cancellationToken = default";

fn path(g: &Gen<'_>, s: &Svc<'_>, file: &str) -> String {
    let mut p = format!("src/{}/Services/", g.target.ns);
    for seg in &s.service_ns {
        p.push_str(seg);
        p.push('/');
    }
    p.push_str(file);
    p
}

/// The namespace alias a child service is written with (`V2::IFileService`), when a service of
/// the same name in an enclosing namespace would hide it.
fn child_alias(g: &Gen<'_>, s: &Svc<'_>, child: &Svc<'_>) -> Option<String> {
    let hidden = g
        .services
        .iter()
        .any(|o| !std::ptr::eq(o, child) && o.prefix == child.prefix && s.service_ns.starts_with(&o.service_ns));
    hidden.then(|| child.service_ns.last().cloned().unwrap_or_default())
}

/// `IFileService` or `V2::IFileService`.
fn child_name(g: &Gen<'_>, s: &Svc<'_>, child: &Svc<'_>, name: &str) -> String {
    child_alias(g, s, child).map_or_else(|| name.to_owned(), |a| format!("{a}::{name}"))
}

/// The usings both files of a service share: the models of its methods and the services of its
/// subresources.
fn shared_usings(g: &Gen<'_>, s: &Svc<'_>) -> Vec<String> {
    let mut u = vec!["System".to_owned(), format!("{}.Core", g.target.ns)];
    if !s.methods.is_empty() {
        u.push("System.Threading".to_owned());
        u.push("System.Threading.Tasks".to_owned());
        u.push(g.models_ns(&s.model_ns));
    }
    for m in &s.methods {
        if let Some(r) = &m.response {
            if r.list {
                u.push("System.Collections.Generic".to_owned());
            }
            match &r.ns {
                Some(ns) if !ns.is_empty() => u.push(g.models_ns(ns)),
                Some(_) => u.push(g.models_ns(&[])),
                None => u.push(format!("Models = {}", g.models_ns(&[]))),
            }
        }
    }
    for &c in &s.children {
        let child = &g.services[c];
        match child_alias(g, s, child) {
            Some(a) => u.push(format!("{a} = {}", g.services_ns(&child.service_ns))),
            None => u.push(g.services_ns(&child.service_ns)),
        }
    }
    u
}

fn ret(m: &Meth<'_>, raw: bool) -> String {
    match (&m.response, raw) {
        (Some(r), false) => format!("Task<{}>", r.spell()),
        (Some(r), true) => format!("Task<HttpResponse<{}>>", r.spell()),
        (None, false) => "Task".to_owned(),
        (None, true) => "Task<HttpResponse>".to_owned(),
    }
}

/// `(XParams parameters, ...)` of the main signature.
fn main_params(m: &Meth<'_>) -> String {
    if m.required {
        format!("{} parameters, {CT}", m.params)
    } else {
        format!("{}? parameters = null, {CT}", m.params)
    }
}

/// `(string id, XParams? parameters = null, ...)` of the overload that takes the last path param.
fn overload_params(m: &Meth<'_>) -> Option<String> {
    let PathParam { arg, ty, .. } = m.path_params.last()?;
    Some(if m.required_beyond_last_path_param {
        format!("{ty} {arg}, {} parameters, {CT}", m.params)
    } else {
        format!("{ty} {arg}, {}? parameters = null, {CT}", m.params)
    })
}

fn sig(ret: &str, name: &str, params: &str) -> String {
    format!("{ret} {name}({params})")
}

fn method_doc(m: &Meth<'_>) -> String {
    let text = if m.method.description.is_empty() {
        &m.method.summary
    } else {
        &m.method.description
    };
    doc::summary(text, 4, doc::Place::Method, &[])
}

/// `I<Name>Service.cs`.
#[must_use]
pub fn interface_file(g: &Gen<'_>, s: &Svc<'_>) -> (String, String) {
    let name = format!("{}Service", s.prefix);
    let iname = format!("I{name}");
    let mut u = shared_usings(g, s);
    u.retain(|x| x != &format!("{}.Exceptions", g.target.ns));
    let mut out = String::new();
    let _ = write!(
        out,
        "{}\nnamespace {};\n\n",
        sort_usings(&u),
        g.services_ns(&s.service_ns)
    );
    out.push_str(&doc::block(
        &doc::NOTE.lines().map(str::to_owned).collect::<Vec<_>>(),
        0,
    ));
    let _ = write!(
        out,
        "public interface {iname}\n{{\n    /// <summary>\n    /// Returns a view of this service that provides access to raw HTTP responses\n    /// for each method.\n    /// </summary>\n    {iname}WithRawResponse WithRawResponse {{ get; }}\n\n    /// <summary>\n    /// Returns a view of this service with the given option modifications applied.\n    ///\n    /// <para>The original service is not modified.</para>\n    /// </summary>\n    {iname} WithOptions(Func<ClientOptions, ClientOptions> modifier);\n"
    );
    for &c in &s.children {
        let child = &g.services[c];
        let _ = write!(
            out,
            "\n    {} {} {{ get; }}\n",
            child_name(g, s, child, &format!("I{}Service", child.prefix)),
            child.prop
        );
    }
    for m in &s.methods {
        out.push('\n');
        out.push_str(&method_doc(m));
        let _ = writeln!(out, "    {};", sig(&ret(m, false), &m.name, &main_params(m)));
        if let Some(o) = overload_params(m) {
            let _ = write!(
                out,
                "\n    /// <inheritdoc cref=\"{}({}, CancellationToken)\"/>\n    {};\n",
                m.name,
                m.params,
                sig(&ret(m, false), &m.name, &o)
            );
        }
    }
    let _ = write!(
        out,
        "}}\n\n/// <summary>\n/// A view of <see cref=\"{iname}\"/> that provides access to raw\n/// HTTP responses for each method.\n/// </summary>\npublic interface {iname}WithRawResponse\n{{\n    /// <summary>\n    /// Returns a view of this service with the given option modifications applied.\n    ///\n    /// <para>The original service is not modified.</para>\n    /// </summary>\n    {iname}WithRawResponse WithOptions(Func<ClientOptions, ClientOptions> modifier);\n"
    );
    for &c in &s.children {
        let child = &g.services[c];
        let _ = write!(
            out,
            "\n    {} {} {{ get; }}\n",
            child_name(g, s, child, &format!("I{}ServiceWithRawResponse", child.prefix)),
            child.prop
        );
    }
    for m in &s.methods {
        let _ = write!(
            out,
            "\n    /// <summary>\n    /// Returns a raw HTTP response for <c>{} {}</c>, but is otherwise the\n    /// same as <see cref=\"{iname}.{}({}{}, CancellationToken)\"/>.\n    /// </summary>\n    {};\n",
            m.method.http_method.as_str(),
            m.method.path,
            m.name,
            m.params,
            if m.required { "" } else { "?" },
            sig(&ret(m, true), &m.name, &main_params(m))
        );
        if let Some(o) = overload_params(m) {
            let _ = write!(
                out,
                "\n    /// <inheritdoc cref=\"{}({}, CancellationToken)\"/>\n    {};\n",
                m.name,
                m.params,
                sig(&ret(m, true), &m.name, &o)
            );
        }
    }
    out.push_str("}\n");
    (path(g, s, &format!("{iname}.cs")), out)
}

fn lazy_field(name: &str, ty: &str) -> String {
    let field = crate::naming::decapitalize(name);
    format!(
        "    readonly Lazy<{ty}> _{field};\n    public {ty} {name}\n    {{\n        get {{ return _{field}.Value; }}\n    }}\n"
    )
}

/// `<Name>Service.cs`.
#[must_use]
pub fn class_file(g: &Gen<'_>, s: &Svc<'_>) -> (String, String) {
    let name = format!("{}Service", s.prefix);
    let iname = format!("I{name}");
    let client = g.iclient();
    let mut u = shared_usings(g, s);
    if !s.methods.is_empty() {
        u.push("System.Net.Http".to_owned());
        if s.methods.iter().any(|m| !m.path_params.is_empty()) {
            u.push(format!("{}.Exceptions", g.target.ns));
        }
    }
    let mut out = String::new();
    let _ = write!(
        out,
        "{}\nnamespace {};\n\n",
        sort_usings(&u),
        g.services_ns(&s.service_ns)
    );
    let _ = write!(
        out,
        "/// <inheritdoc/>\npublic sealed class {name} : {iname}\n{{\n    readonly Lazy<{iname}WithRawResponse> _withRawResponse;\n\n    /// <inheritdoc/>\n    public {iname}WithRawResponse WithRawResponse\n    {{\n        get {{ return _withRawResponse.Value; }}\n    }}\n\n    readonly {client} _client;\n\n    /// <inheritdoc/>\n    public {iname} WithOptions(Func<ClientOptions, ClientOptions> modifier)\n    {{\n        return new {name}(this._client.WithOptions(modifier));\n    }}\n\n    public {name}({client} client)\n    {{\n        _client = client;\n\n        _withRawResponse = new(() => new {name}WithRawResponse(client.WithRawResponse));\n"
    );
    for &c in &s.children {
        let child = &g.services[c];
        let _ = writeln!(
            out,
            "        _{} = new(() => new {}(client));",
            crate::naming::decapitalize(&child.prop),
            child_name(g, s, child, &format!("{}Service", child.prefix))
        );
    }
    out.push_str("    }\n");
    for &c in &s.children {
        let child = &g.services[c];
        out.push('\n');
        out.push_str(&lazy_field(
            &child.prop,
            &child_name(g, s, child, &format!("I{}Service", child.prefix)),
        ));
    }
    for m in &s.methods {
        out.push('\n');
        out.push_str(&service_method(m));
    }
    let _ = write!(
        out,
        "}}\n\n/// <inheritdoc/>\npublic sealed class {name}WithRawResponse : {iname}WithRawResponse\n{{\n    readonly {client}WithRawResponse _client;\n\n    /// <inheritdoc/>\n    public {iname}WithRawResponse WithOptions(Func<ClientOptions, ClientOptions> modifier)\n    {{\n        return new {name}WithRawResponse(this._client.WithOptions(modifier));\n    }}\n\n    public {name}WithRawResponse({client}WithRawResponse client)\n    {{\n        _client = client;\n"
    );
    if !s.children.is_empty() {
        out.push('\n');
    }
    for &c in &s.children {
        let child = &g.services[c];
        let _ = writeln!(
            out,
            "        _{} = new(() => new {}(client));",
            crate::naming::decapitalize(&child.prop),
            child_name(g, s, child, &format!("{}ServiceWithRawResponse", child.prefix))
        );
    }
    out.push_str("    }\n");
    for &c in &s.children {
        let child = &g.services[c];
        out.push('\n');
        out.push_str(&lazy_field(
            &child.prop,
            &child_name(g, s, child, &format!("I{}ServiceWithRawResponse", child.prefix)),
        ));
    }
    for m in &s.methods {
        out.push('\n');
        out.push_str(&raw_method(g, m));
    }
    out.push_str("}\n");
    (path(g, s, &format!("{name}.cs")), out)
}

/// The overload's body: `this.Get(parameters with { ID = id }, cancellationToken)`.
fn with_path(m: &Meth<'_>) -> String {
    let PathParam { prop, arg, .. } = m.path_params.last().cloned().unwrap_or_default();
    format!(
        "this.{}(parameters with {{ {prop} = {arg} }}, cancellationToken)",
        m.name
    )
}

fn service_method(m: &Meth<'_>) -> String {
    let mut out = String::new();
    let main = main_params(m);
    if m.response.is_some() {
        let _ = write!(
            out,
            "    /// <inheritdoc/>\n    public async {} {}({main})\n    {{\n        using var response = await this\n            .WithRawResponse.{}(parameters, cancellationToken)\n            .ConfigureAwait(false);\n        return await response.Deserialize(cancellationToken).ConfigureAwait(false);\n    }}\n",
            ret(m, false),
            m.name,
            m.name
        );
    } else {
        let _ = write!(
            out,
            "    /// <inheritdoc/>\n    public {} {}({main})\n    {{\n        return this.WithRawResponse.{}(parameters, cancellationToken);\n    }}\n",
            ret(m, false),
            m.name,
            m.name
        );
    }
    if let Some(o) = overload_params(m) {
        let default = if m.required_beyond_last_path_param {
            ""
        } else {
            "        parameters ??= new();\n\n"
        };
        if m.response.is_some() {
            let _ = write!(
                out,
                "\n    /// <inheritdoc/>\n    public {} {}({o})\n    {{\n{default}        return {};\n    }}\n",
                ret(m, false),
                m.name,
                with_path(m)
            );
        } else {
            let _ = write!(
                out,
                "\n    /// <inheritdoc/>\n    public async {} {}({o})\n    {{\n{default}        await {}.ConfigureAwait(false);\n    }}\n",
                ret(m, false),
                m.name,
                with_path(m)
            );
        }
    }
    out
}

fn http_method(g: &Gen<'_>, m: &Meth<'_>) -> String {
    let verb = m.method.http_method.as_str();
    if verb == "patch" {
        format!("{}ClientWithRawResponse.PatchMethod", g.target.client)
    } else {
        format!("HttpMethod.{}", crate::naming::capitalize(verb))
    }
}

fn raw_method(g: &Gen<'_>, m: &Meth<'_>) -> String {
    let mut out = String::new();
    let main = main_params(m);
    let asynchronous = if m.response.is_some() { "async " } else { "" };
    let _ = write!(
        out,
        "    /// <inheritdoc/>\n    public {asynchronous}{} {}({main})\n    {{\n",
        ret(m, true),
        m.name
    );
    if !m.required {
        out.push_str("        parameters ??= new();\n\n");
    }
    if let Some(PathParam { prop, .. }) = m.path_params.last() {
        let _ = write!(
            out,
            "        if (parameters.{prop} == null)\n        {{\n            throw new {}InvalidDataException(\"'parameters.{prop}' cannot be null\");\n        }}\n\n",
            g.target.client
        );
    }
    let _ = write!(
        out,
        "        HttpRequest<{}> request = new()\n        {{\n            Method = {},\n            Params = parameters,\n        }};\n",
        m.params,
        http_method(g, m)
    );
    match &m.response {
        None => out.push_str("        return this._client.Execute(request, cancellationToken);\n"),
        Some(r) => {
            let var = &r.var;
            let validate = if r.list {
                format!("foreach (var item in {var})\n{{\nitem.Validate();\n}}\n")
            } else {
                format!("{var}.Validate();\n")
            };
            let _ = write!(
                out,
                "        var response = await this._client.Execute(request, cancellationToken).ConfigureAwait(false);\n        return new(\n            response,\n            async (token) =>\n            {{\n                var {var} = await response\n                    .Deserialize<{}>(token)\n                    .ConfigureAwait(false);\n                if (this._client.ResponseValidation)\n                {{\n{validate}                }}\n                return {var};\n            }}\n        );\n",
                r.spell()
            );
        }
    }
    out.push_str("    }\n");
    if let Some(o) = overload_params(m) {
        let default = if m.required_beyond_last_path_param {
            ""
        } else {
            "        parameters ??= new();\n\n"
        };
        let _ = write!(
            out,
            "\n    /// <inheritdoc/>\n    public {} {}({o})\n    {{\n{default}        return {};\n    }}\n",
            ret(m, true),
            m.name,
            with_path(m)
        );
    }
    out
}
