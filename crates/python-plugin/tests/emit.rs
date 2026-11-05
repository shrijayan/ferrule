//! Emitter unit tests on hand-built models.

use ferrule_python_plugin::diagnostics::FerCode;
use ferrule_python_plugin::emit::client::emit_client_file;
use ferrule_python_plugin::emit::common::{args_docstring, stainless_method_order};
use ferrule_python_plugin::emit::files::input::{AuthScheme, ExampleRequest, ParamLocation as FilesLocation};
use ferrule_python_plugin::emit::files_input::{ProjectMeta, ReadmeRequest, placeholder_meta, to_project_files_input};
use ferrule_python_plugin::emit::params::emit_params_file;
use ferrule_python_plugin::emit::project::emit_root_files;
use ferrule_python_plugin::emit::resources::emit_resource_file;
use ferrule_python_plugin::model::{
    ApiKeyLocation, ArrayFormat, AuthRole, HttpAuthScheme, HttpVerb, NestedFormat, OptType, PaginationScheme,
    ParamLocation, SdkAuth, SdkClient, SdkClientOpt, SdkClientSettings, SdkEnvironment, SdkField, SdkLiteral,
    SdkMethod, SdkModelDecl, SdkObject, SdkPackage, SdkPagination, SdkParam, SdkPrimitive, SdkRef, SdkResource,
    SdkSecurityScheme, SdkSend, SdkStreaming, SdkStyle, SdkType, SendLocation, StreamFormat,
};

fn str_t() -> SdkType {
    SdkType::Primitive(SdkPrimitive::String)
}

fn field(wire: &str, py: &str, required: bool, description: &str, ty: SdkType) -> SdkField {
    SdkField {
        wire_name: wire.into(),
        py_name: py.into(),
        required,
        description: description.into(),
        ty,
    }
}

/// An object with one required string field `name`.
fn obj_with(name: &str) -> SdkObject {
    SdkObject {
        fields: vec![field(name, name, true, "", str_t())],
        ..SdkObject::default()
    }
}

fn param(wire: &str, py: &str) -> SdkParam {
    SdkParam {
        wire_name: wire.into(),
        py_name: py.into(),
        required: false,
        description: String::new(),
        ty: str_t(),
        deprecated: false,
        example: None,
        minimum: None,
        maximum: None,
    }
}

fn as_param(f: &SdkField) -> SdkParam {
    SdkParam {
        required: f.required,
        description: f.description.clone(),
        ty: f.ty.clone(),
        ..param(&f.wire_name, &f.py_name)
    }
}

fn method(response: Option<SdkType>) -> SdkMethod {
    SdkMethod {
        name: "list".into(),
        pointer: "#/paths/~1t/get".into(),
        http_method: HttpVerb::Get,
        path: "/t".into(),
        summary: String::new(),
        description: String::new(),
        order: 0,
        deprecated: false,
        path_params: Vec::new(),
        query_params: Vec::new(),
        header_params: Vec::new(),
        body: None,
        body_params: Vec::new(),
        body_media_type: None,
        body_whole: false,
        body_binary: false,
        body_variants: Vec::new(),
        body_discriminator: None,
        body_file_paths: Vec::new(),
        response,
        unwrap: None,
        skip_test: None,
        pagination: None,
        streaming: None,
    }
}

fn resource(path: &[&str], methods: Vec<SdkMethod>) -> SdkResource {
    SdkResource {
        path: path.iter().map(|&p| p.to_owned()).collect(),
        order: 0,
        methods,
        subresources: Vec::new(),
        class_segment: None,
        singular_class_segment: None,
        standalone: false,
    }
}

fn settings() -> SdkClientSettings {
    SdkClientSettings {
        name: "acme".into(),
        class_name: "Acme".into(),
        env_prefix: "ACME".into(),
        opts: vec![SdkClientOpt {
            name: "api_key".into(),
            ty: OptType::String,
            nullable: false,
            default: None,
            read_env: Some("ACME_API_KEY".into()),
            description: None,
            auth: Some(SdkAuth {
                security_scheme: "Bearer".into(),
                role: AuthRole::Value,
            }),
            send: None,
        }],
        environments: vec![SdkEnvironment {
            name: "production".into(),
            url: "https://api.acme.test".into(),
        }],
        security_schemes: vec![(
            "Bearer".into(),
            SdkSecurityScheme::Http {
                scheme: HttpAuthScheme::Bearer,
                keyword: None,
            },
        )],
        security: vec![vec!["Bearer".into()]],
        timeout_seconds: 60.0,
        max_retries: 2,
        idempotency_header: None,
        default_headers: Vec::new(),
        array_format: ArrayFormat::Comma,
        nested_format: NestedFormat::Brackets,
    }
}

fn client(resources: Vec<SdkResource>, settings: Option<SdkClientSettings>, repo: &str) -> SdkClient {
    SdkClient {
        package: SdkPackage {
            name: "acme".into(),
            repo: repo.into(),
        },
        base_url: String::new(),
        settings,
        resources,
        client_methods: Vec::new(),
        client_class: None,
        env_prefix: None,
        models: Vec::new(),
        resource_tree: Vec::new(),
        style: SdkStyle::default(),
    }
}

fn plain_client(m: SdkMethod) -> SdkClient {
    client(vec![resource(&["things"], vec![m])], None, "a/b")
}

fn sdk(methods: Vec<SdkMethod>) -> SdkClient {
    client(
        vec![resource(&["things"], methods)],
        Some(settings()),
        "acme/acme-python",
    )
}

fn first(c: &SdkClient) -> &SdkResource {
    &c.resources[0]
}

#[test]
fn awaits_async_maybe_transform_for_a_non_paginated_async_method_with_query() {
    let mut m = method(Some(SdkType::Object(obj_with("x"))));
    m.http_method = HttpVerb::Post;
    m.description = "Does things.".into();
    m.query_params = vec![
        SdkParam {
            required: true,
            description: "Q.".into(),
            ..param("q", "q")
        },
        param("r", "r"),
    ];
    let c = plain_client(m);
    let out = emit_resource_file(&c, first(&c)).expect("emits");
    assert!(out.contains("query=await async_maybe_transform("));
    assert!(out.contains("return await self._post("));
    assert!(out.contains("        q: str,\n"));
    assert!(out.contains("          q: Q.\n\n          extra_headers:"));
}

#[test]
fn prints_a_docstring_less_method() {
    let c = plain_client(method(Some(SdkType::Object(obj_with("x")))));
    let out = emit_resource_file(&c, first(&c)).expect("emits");
    assert!(out.contains(") -> ThingListResponse:\n        return self._get("));
}

#[test]
fn rejects_a_body_that_is_not_an_object_and_param_switched_streaming() {
    let mut a = method(Some(SdkType::Object(obj_with("x"))));
    a.http_method = HttpVerb::Post;
    a.body = Some(str_t());
    let c = plain_client(a);
    assert_eq!(
        emit_resource_file(&c, first(&c)).map_err(|e| e.code),
        Err(FerCode::Fer201)
    );
    let mut b = method(Some(SdkType::Object(obj_with("x"))));
    b.streaming = Some(SdkStreaming {
        format: StreamFormat::Sse,
        param_discriminator: Some("stream".into()),
        event: Some(SdkType::Object(obj_with("x"))),
    });
    let c = plain_client(b);
    assert_eq!(
        emit_resource_file(&c, first(&c)).map_err(|e| e.code),
        Err(FerCode::Fer201)
    );
}

#[test]
fn emits_nested_resources_as_packages_and_aliases_non_object_responses() {
    let child = resource(&["p", "things"], vec![method(Some(SdkType::Array(Box::new(str_t()))))]);
    let mut parent = resource(&["p"], Vec::new());
    parent.subresources = vec![child];
    let files = emit_root_files(&client(vec![parent], None, "a/b"), None).expect("emits");
    let paths: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(
        paths,
        [
            "src/acme/resources/__init__.py",
            "src/acme/resources/p/__init__.py",
            "src/acme/resources/p/p.py",
            "src/acme/resources/p/things.py",
            "src/acme/types/__init__.py",
            "src/acme/types/p/__init__.py",
            "src/acme/types/p/thing_list_response.py",
        ]
    );
    let alias = &files
        .iter()
        .find(|(p, _)| p.ends_with("thing_list_response.py"))
        .expect("file")
        .1;
    assert!(alias.contains("ThingListResponse: TypeAlias = List[str]"));
}

#[test]
fn marks_required_params_and_imports_named_models() {
    let r = resource(&["things"], Vec::new());
    let mut m = method(Some(SdkType::Object(obj_with("x"))));
    m.query_params = vec![
        SdkParam {
            required: true,
            ..param("q", "q")
        },
        SdkParam {
            ty: SdkType::Ref(SdkRef {
                name: "U".into(),
                module: "shared.u".into(),
            }),
            ..param("u", "u")
        },
    ];
    let out = emit_params_file(Some(&r), &m);
    assert!(out.contains("q: Required[str]"));
    // A request takes the model's `TypedDict` twin (`tests/param_twins`).
    assert!(out.contains("from .shared_params.u import U"));
}

#[test]
fn aliases_renamed_params_and_declares_inline_typed_dicts() {
    let r = resource(&["things"], Vec::new());
    let mut m = method(Some(SdkType::Object(obj_with("x"))));
    m.query_params = vec![
        param("qQ", "q_q"),
        SdkParam {
            ty: SdkType::Object(obj_with("x")),
            ..param("q", "q")
        },
    ];
    let out = emit_params_file(Some(&r), &m);
    assert!(out.contains("q_q: Annotated[str, PropertyInfo(alias=\"qQ\")]"));
    assert!(out.contains("class Q(TypedDict, total=False):"));
}

#[test]
fn emits_bearer_auth_a_client_method_and_default_headers() {
    let mut c = sdk(vec![method(Some(SdkType::Object(obj_with("id"))))]);
    let mut health = method(None);
    health.name = "health".into();
    c.client_methods = vec![health];
    if let Some(s) = c.settings.as_mut() {
        s.default_headers = vec![("X-Version".into(), "2".into())];
    }
    let out = emit_client_file(&c).expect("emits");
    assert!(out.contains("return {\"Authorization\": f\"Bearer {api_key}\"}"));
    assert!(out.contains("\"X-Version\": \"2\","));
    assert!(out.contains("    def health(\n"));
    assert!(out.contains("class AsyncAcmeWithStreamedResponse:"));
    assert!(out.contains("Client = Acme\n\nAsyncClient = AsyncAcme\n"));
}

#[test]
fn emits_header_api_keys_optional_and_several_env_options() {
    let mut c = sdk(vec![method(Some(SdkType::Object(obj_with("id"))))]);
    let s = c.settings.as_mut().expect("settings");
    let opt = s.opts[0].clone();
    s.opts = vec![
        SdkClientOpt {
            auth: Some(SdkAuth {
                security_scheme: "H".into(),
                role: AuthRole::Value,
            }),
            ..opt.clone()
        },
        SdkClientOpt {
            name: "org".into(),
            nullable: true,
            read_env: Some("ACME_ORG".into()),
            auth: None,
            ..opt.clone()
        },
        SdkClientOpt {
            name: "region".into(),
            read_env: None,
            auth: None,
            ..opt
        },
    ];
    s.security_schemes = vec![(
        "H".into(),
        SdkSecurityScheme::ApiKey {
            location: ApiKeyLocation::Header,
            name: "X-Key".into(),
        },
    )];
    s.security = vec![vec!["Missing".into(), "H".into()]];
    let out = emit_client_file(&c).expect("emits");
    assert!(out.contains("return {\"X-Key\": api_key}"));
    assert!(out.contains("org: str | None\n"));
    assert!(out.contains("- `org` from `ACME_ORG`"));
    assert!(out.contains("by passing region to the client\""));
}

/// A client option with a `default` (the opencode SDK's basic-auth `username: opencode`) takes it
/// when neither the argument nor its environment variable gives one, before the required check.
#[test]
fn applies_a_client_option_default_after_the_environment() {
    let mut c = sdk(vec![method(Some(SdkType::Object(obj_with("id"))))]);
    let s = c.settings.as_mut().expect("settings");
    let opt = s.opts[0].clone();
    s.opts = vec![
        SdkClientOpt {
            name: "username".into(),
            nullable: true,
            default: Some(serde_json::json!("opencode")),
            read_env: Some("ACME_USERNAME".into()),
            auth: None,
            ..opt.clone()
        },
        SdkClientOpt {
            name: "region".into(),
            default: Some(serde_json::json!("us")),
            read_env: None,
            auth: None,
            ..opt
        },
    ];
    let out = emit_client_file(&c).expect("emits");
    let want = "        if username is None:\n            username = os.environ.get(\"ACME_USERNAME\")\n        \
                if username is None:\n            username = \"opencode\"\n        self.username = username\n";
    assert!(out.contains(want), "{out}");
    assert!(
        out.contains("        if region is None:\n            region = \"us\"\n"),
        "{out}"
    );
    assert!(!out.contains("by passing region to the client"), "{out}");
}

/// HTTP basic auth (the opencode SDK: `username` and `password` options with those roles) is the
/// `Authorization: Basic` header of `username:password`, as modern-treasury-python's
/// `auth_headers` writes it, and nothing while a nullable option is unset.
#[test]
fn emits_basic_auth_from_the_username_and_password_options() {
    let mut c = sdk(vec![method(Some(SdkType::Object(obj_with("id"))))]);
    let s = c.settings.as_mut().expect("settings");
    let opt = s.opts[0].clone();
    let role = |role| {
        Some(SdkAuth {
            security_scheme: "basicAuth".into(),
            role,
        })
    };
    s.opts = vec![
        SdkClientOpt {
            name: "username".into(),
            nullable: true,
            read_env: None,
            auth: role(AuthRole::Username),
            ..opt.clone()
        },
        SdkClientOpt {
            name: "password".into(),
            nullable: true,
            read_env: Some("ACME_PASSWORD".into()),
            auth: role(AuthRole::Password),
            ..opt
        },
    ];
    s.security_schemes = vec![(
        "basicAuth".into(),
        SdkSecurityScheme::Http {
            scheme: HttpAuthScheme::Basic,
            keyword: None,
        },
    )];
    s.security = vec![vec!["basicAuth".into()]];
    let out = emit_client_file(&c).expect("emits");
    let want = "    def auth_headers(self) -> dict[str, str]:
        if self.username is None or self.password is None:
            return {}
        credentials = f\"{self.username}:{self.password}\".encode(\"ascii\")
        header = f\"Basic {base64.b64encode(credentials).decode('ascii')}\"
        return {\"Authorization\": header}
";
    assert!(out.contains(want), "{out}");
    assert!(out.contains("\nimport base64\n"), "{out}");
}

/// Several auth schemes (papr-python `_client.py`: an API key header, a nullable session token
/// header, a nullable bearer token, each its own security requirement): `auth_headers` merges
/// one property per scheme, the last requirement first, so the first one wins.
#[test]
fn merges_a_property_per_auth_scheme() {
    let mut c = sdk(vec![method(Some(SdkType::Object(obj_with("id"))))]);
    let s = c.settings.as_mut().expect("settings");
    let opt = |name: &str, nullable: bool, scheme: &str| SdkClientOpt {
        name: name.into(),
        nullable,
        auth: Some(SdkAuth {
            security_scheme: scheme.into(),
            role: AuthRole::Value,
        }),
        ..s.opts[0].clone()
    };
    s.opts = vec![
        opt("x_api_key", false, "X-API-Key"),
        opt("x_session_token", true, "X-Session-Token"),
        opt("bearer_token", true, "Bearer"),
    ];
    let header = |name: &str| SdkSecurityScheme::ApiKey {
        location: ApiKeyLocation::Header,
        name: name.into(),
    };
    s.security_schemes = vec![
        ("X-API-Key".into(), header("X-API-Key")),
        ("X-Session-Token".into(), header("X-Session-Token")),
        (
            "Bearer".into(),
            SdkSecurityScheme::Http {
                scheme: HttpAuthScheme::Bearer,
                keyword: None,
            },
        ),
    ];
    s.security = vec![
        vec!["X-API-Key".into()],
        vec!["X-Session-Token".into()],
        vec!["Bearer".into()],
    ];
    let out = emit_client_file(&c).expect("emits");
    let expected = r#"    @property
    @override
    def auth_headers(self) -> dict[str, str]:
        return {**self._bearer, **self._x_session_token, **self._x_api_key}

    @property
    def _bearer(self) -> dict[str, str]:
        bearer_token = self.bearer_token
        if bearer_token is None:
            return {}
        return {"Authorization": f"Bearer {bearer_token}"}

    @property
    def _x_session_token(self) -> dict[str, str]:
        x_session_token = self.x_session_token
        if x_session_token is None:
            return {}
        return {"X-Session-Token": x_session_token}

    @property
    def _x_api_key(self) -> dict[str, str]:
        x_api_key = self.x_api_key
        return {"X-API-Key": x_api_key}

    @property
    @override
    def default_headers(self) -> dict[str, str | Omit]:
"#;
    assert_eq!(out.matches(expected).count(), 2, "{out}");
}

#[test]
fn falls_back_to_no_auth_headers_and_rejects_what_it_cannot_emit() {
    let mut c = sdk(vec![method(Some(SdkType::Object(obj_with("id"))))]);
    c.settings.as_mut().expect("settings").security = Vec::new();
    assert!(
        emit_client_file(&c)
            .expect("emits")
            .contains("def auth_headers(self) -> dict[str, str]:\n        return {}")
    );
    let mut many = c.clone();
    many.settings.as_mut().expect("settings").environments = vec![
        SdkEnvironment {
            name: "a".into(),
            url: "https://a".into(),
        },
        SdkEnvironment {
            name: "b".into(),
            url: "https://b".into(),
        },
    ];
    // Several environments are emitted now (tests/environments.rs pins the text).
    assert!(
        emit_client_file(&many)
            .expect("emits")
            .contains("self._environment = environment = \"a\"")
    );
    c.settings = None;
    assert_eq!(emit_client_file(&c).map_err(|e| e.code), Err(FerCode::Fer103));
}

#[test]
fn sends_bodies_path_params_headers_and_no_content_responses() {
    let body = SdkObject {
        fields: vec![
            field("name", "name", true, "The name.", str_t()),
            field("tags", "tags", false, "", SdkType::Array(Box::new(str_t()))),
        ],
        ..SdkObject::default()
    };
    let mut m = method(None);
    m.name = "update".into();
    m.http_method = HttpVerb::Post;
    m.path = "/t/{thingID}".into();
    m.path_params = vec![SdkParam {
        required: true,
        ..param("thingID", "thing_id")
    }];
    m.query_params = vec![param("dryRun", "dry_run")];
    m.body_params = body.fields.iter().map(as_param).collect();
    m.body = Some(SdkType::Object(body));
    let c = sdk(vec![m]);
    let out = emit_resource_file(&c, first(&c)).expect("emits");
    assert!(out.contains("path_template(\"/t/{thing_id}\", thing_id=thing_id)"));
    assert!(out.contains("raise ValueError(f\"Expected a non-empty value for `thing_id`"));
    assert!(out.contains("tags: SequenceNotStr[str] | Omit = omit,"));
    assert!(out.contains("body=await async_maybe_transform("));
    assert!(out.contains("cast_to=NoneType,"));
    assert!(out.contains("extra_headers = {\"Accept\": \"*/*\", **(extra_headers or {})}"));
    let params = emit_params_file(Some(first(&c)), &first(&c).methods[0]);
    assert!(params.contains("class ThingUpdateParams(TypedDict, total=False):"));
}

#[test]
fn pages_over_get_and_post_and_streams_events() {
    let page = SdkPagination {
        scheme: PaginationScheme::Cursor,
        page: "CursorPage".into(),
        request: Vec::new(),
        response: vec![("data".into(), "items".into())],
        param_location: ParamLocation::Body,
        headers: Vec::new(),
        item: SdkType::Object(obj_with("id")),
    };
    let mut list = method(Some(SdkType::Object(obj_with("id"))));
    list.pagination = Some(page.clone());
    let mut search = method(Some(SdkType::Object(obj_with("id"))));
    search.name = "search".into();
    search.http_method = HttpVerb::Post;
    search.pagination = Some(page);
    search.body_params = vec![param("q", "q")];
    search.body = Some(SdkType::Object(obj_with("id")));
    let mut events = method(Some(SdkType::Object(obj_with("id"))));
    events.name = "events".into();
    events.streaming = Some(SdkStreaming {
        format: StreamFormat::Sse,
        param_discriminator: None,
        event: Some(SdkType::Object(obj_with("id"))),
    });
    let c = sdk(vec![list, search, events]);
    let out = emit_resource_file(&c, first(&c)).expect("emits");
    assert!(out.contains("-> SyncCursorPage[ThingListResponse]:"));
    assert!(out.contains("-> AsyncPaginator[ThingSearchResponse, AsyncCursorPage[ThingSearchResponse]]:"));
    assert!(out.contains("method=\"post\","));
    assert!(out.contains("stream_cls=AsyncStream[ThingEventsResponse],"));
    let order: Vec<&str> = stainless_method_order(&first(&c).methods)
        .iter()
        .map(|m| m.name.as_str())
        .collect();
    assert_eq!(order, ["list", "events", "search"]);
}

#[test]
fn builds_nested_resource_packages_with_subresource_properties() {
    let child = resource(
        &["things", "parts"],
        vec![method(Some(SdkType::Object(obj_with("id"))))],
    );
    let mut parent = resource(&["things"], vec![method(Some(SdkType::Object(obj_with("id"))))]);
    parent.subresources = vec![child];
    let c = client(vec![parent], Some(settings()), "acme/acme-python");
    let files = emit_root_files(&c, None).expect("emits");
    let get = |path: &str| files.iter().find(|(p, _)| p == path).map(|(_, t)| t.as_str());
    let pkg = get("src/acme/resources/things/things.py").expect("package module");
    assert!(pkg.contains("def parts(self) -> PartsResource:\n        return PartsResource(self._client)"));
    assert!(pkg.contains("return AsyncPartsResourceWithStreamingResponse(self._things.parts)"));
    assert!(
        get("src/acme/resources/things/__init__.py")
            .expect("init")
            .contains("\"PartsResource\",")
    );
    assert!(get("src/acme/types/things/part_list_response.py").is_some());
    assert!(get("src/acme/_client.py").is_some());
}

#[test]
fn names_param_union_variants_after_their_schemas() {
    let variant = |hint: &str, tag: &str| {
        SdkType::Object(SdkObject {
            hint: Some(hint.into()),
            description: Some(format!("{hint} object")),
            fields: vec![field(
                "type",
                "type",
                true,
                "",
                SdkType::Literal(vec![SdkLiteral::String(tag.into())]),
            )],
            ..SdkObject::default()
        })
    };
    let union = SdkType::Union {
        variants: vec![variant("ImageRef", "image"), variant("VideoRef", "video")],
        discriminator: None,
    };
    let mut m = method(Some(SdkType::Object(obj_with("id"))));
    m.http_method = HttpVerb::Post;
    m.body_params = vec![
        SdkParam {
            ty: union,
            ..param("frame", "frame")
        },
        SdkParam {
            ty: SdkType::Primitive(SdkPrimitive::Date),
            required: true,
            ..param("date", "date")
        },
    ];
    m.body = Some(SdkType::Object(obj_with("id")));
    let r = resource(&["things"], Vec::new());
    let out = emit_params_file(Some(&r), &m);
    assert!(
        out.contains("Frame: TypeAlias = Union[FrameImageRef, FrameVideoRef]"),
        "{out}"
    );
    assert!(out.contains("class FrameImageRef(TypedDict, total=False):\n    \"\"\"ImageRef object\"\"\""));
    assert!(out.contains("date: Required[Annotated[Union[str, datetime.date], PropertyInfo(format=\"iso8601\")]]"));
}

#[test]
fn documents_documented_arguments_only() {
    let mut m = method(None);
    m.summary = "Sum.".into();
    let d = SdkParam {
        description: "D.".into(),
        ..param("q", "q")
    };
    let u = param("q", "u");
    assert!(args_docstring(&m, Some(&[&d, &u])).contains(&"  q: D.".to_owned()));
    let mut m = method(None);
    m.description = "One.\nTwo.".into();
    assert_eq!(args_docstring(&m, None), ["\"\"\"", "One.", "Two.", "\"\"\""]);
}

#[test]
fn maps_resources_params_and_the_api_key_query_auth() {
    let mut m = method(Some(SdkType::Object(obj_with("id"))));
    m.path_params = vec![SdkParam {
        required: true,
        ..param("id", "id")
    }];
    m.query_params = vec![SdkParam {
        example: Some(serde_json::json!("x")),
        minimum: Some(1.0),
        maximum: Some(2.0),
        ..param("q", "q")
    }];
    let key = SdkClientOpt {
        send: Some(SdkSend {
            location: SendLocation::Query,
            name: "key".into(),
        }),
        ..settings().opts[0].clone()
    };
    let with_key = SdkClientSettings {
        opts: vec![key],
        ..settings()
    };
    let meta = placeholder_meta();
    // The only method has a path parameter, so there is no README example.
    let c = client(
        vec![resource(&["things"], vec![m.clone()])],
        Some(with_key.clone()),
        "acme/acme-python",
    );
    assert!(to_project_files_input(&c, &meta).is_none());
    let free = client(
        vec![resource(
            &["things"],
            vec![method(Some(SdkType::Object(obj_with("id")))), m],
        )],
        Some(with_key),
        "acme/acme-python",
    );
    let input = to_project_files_input(&free, &meta).expect("an input");
    assert_eq!(input.auth.scheme, AuthScheme::Query("key".to_owned()));
    assert_eq!(
        input.auth.opts.iter().map(|o| o.env.as_deref()).collect::<Vec<_>>(),
        [Some("ACME_API_KEY")]
    );
    assert_eq!(input.readme_example.accessor, "things");
    // TypeScript expects `[]` here: both methods are named `list`, and the first has no params.
    let params: Vec<(FilesLocation, Option<serde_json::Value>)> = input.resources[0].methods[0]
        .params
        .iter()
        .map(|p| (p.location, p.example.clone()))
        .collect();
    assert_eq!(params, Vec::new());
    assert_eq!(input.resources[0].variable, "thing");
    // The templates are today's: knock-python's `tests/test_transform.py` imports `NOT_GIVEN`
    // and tests `PYDANTIC_V2`, so a `NOT_GIVEN` runtime gets no project files.
    let mut not_given = free.clone();
    not_given.style = SdkStyle {
        omit: false,
        ..SdkStyle::default()
    };
    assert!(to_project_files_input(&not_given, &meta).is_none());
    let mut no_settings = free;
    no_settings.settings = None;
    assert!(to_project_files_input(&no_settings, &meta).is_none());
}

#[test]
fn emits_project_files_for_a_bearer_token_client() {
    // lumaai, isaacus and knock-mgmt send a bearer token, not a query parameter. Without
    // pyproject.toml, ruff's default isort turns the runtime's `_utils` imports into a cycle and
    // the SDK's own tests cannot even be collected.
    let c = client(
        vec![resource(
            &["things"],
            vec![method(Some(SdkType::Object(obj_with("id"))))],
        )],
        Some(settings()),
        "acme/acme-python",
    );
    assert!(to_project_files_input(&c, &placeholder_meta()).is_some());
    let files = emit_root_files(&c, Some(&placeholder_meta())).expect("emits");
    for path in [
        "pyproject.toml",
        "README.md",
        "tests/test_client.py",
        "tests/conftest.py",
    ] {
        assert!(files.iter().any(|(p, _)| p == path), "no {path}");
    }
}

fn auth_opt(name: &str, scheme: &str, role: AuthRole) -> SdkClientOpt {
    SdkClientOpt {
        name: name.into(),
        read_env: Some(format!("ACME_{}", name.to_uppercase())),
        auth: Some(SdkAuth {
            security_scheme: scheme.into(),
            role,
        }),
        ..settings().opts[0].clone()
    }
}

/// A client with one method that needs no arguments, authenticated by `opts` and `schemes`.
fn authed(opts: Vec<SdkClientOpt>, schemes: Vec<(String, SdkSecurityScheme)>) -> SdkClient {
    client(
        vec![resource(
            &["things"],
            vec![method(Some(SdkType::Object(obj_with("id"))))],
        )],
        Some(SdkClientSettings {
            opts,
            security_schemes: schemes,
            ..settings()
        }),
        "acme/acme-python",
    )
}

fn project_file(c: &SdkClient, path: &str) -> String {
    emit_root_files(c, Some(&placeholder_meta()))
        .expect("emits")
        .into_iter()
        .find(|(p, _)| p == path)
        .map_or_else(|| panic!("no {path}"), |(_, t)| t)
}

fn basic() -> SdkClient {
    let mut password = auth_opt("password", "basicAuth", AuthRole::Password);
    password.default = Some(serde_json::json!("do_not_set"));
    let webhook_secret = SdkClientOpt {
        auth: None,
        ..auth_opt("webhook_secret", "basicAuth", AuthRole::Value)
    };
    authed(
        vec![
            auth_opt("private_key", "basicAuth", AuthRole::Username),
            password,
            webhook_secret,
        ],
        vec![(
            "basicAuth".into(),
            SdkSecurityScheme::Http {
                scheme: HttpAuthScheme::Basic,
                keyword: None,
            },
        )],
    )
}

fn header() -> SdkClient {
    authed(
        vec![auth_opt("x_api_key", "X-API-Key", AuthRole::Value)],
        vec![(
            "X-API-Key".into(),
            SdkSecurityScheme::ApiKey {
                location: ApiKeyLocation::Header,
                name: "X-API-Key".into(),
            },
        )],
    )
}

#[test]
fn derives_the_auth_of_each_scheme() {
    let input = |c: &SdkClient| to_project_files_input(c, &placeholder_meta()).expect("an input");
    let bearer = input(&authed(settings().opts, settings().security_schemes));
    assert_eq!(bearer.auth.scheme, AuthScheme::Bearer);
    let opt = &bearer.auth.opts[0];
    assert_eq!(
        (opt.name.as_str(), opt.value.as_str(), opt.label.as_str()),
        ("api_key", "My API Key", "API Key")
    );
    // imagekit: the options with `auth`, in config order; `webhook_secret` has none.
    let basic = input(&basic());
    assert_eq!(basic.auth.scheme, AuthScheme::Basic);
    let opts: Vec<(&str, &str, bool)> = basic
        .auth
        .opts
        .iter()
        .map(|o| (o.name.as_str(), o.value.as_str(), o.has_default))
        .collect();
    assert_eq!(
        opts,
        [
            ("private_key", "My Private Key", false),
            ("password", "My Password", true)
        ]
    );
    assert_eq!(input(&header()).auth.scheme, AuthScheme::Header("X-API-Key".to_owned()));
    let none = input(&authed(Vec::new(), Vec::new()));
    assert_eq!((none.auth.scheme, none.auth.opts.len()), (AuthScheme::None, 0));
}

#[test]
fn writes_each_scheme_into_the_tests_and_readme() {
    let tests = project_file(&basic(), "tests/test_client.py");
    assert!(tests.contains("private_key = \"My Private Key\"\npassword = \"My Password\"\n"));
    assert!(tests.contains("assert \"Basic\" in request.headers.get(\"Authorization\")"));
    assert!(tests.contains(
        "            with update_env(\n                **{\n                    \"ACME_PRIVATE_KEY\": Omit(),\n                    \"ACME_PASSWORD\": Omit(),\n                }\n            ):"
    ));
    // The README leaves out an option with a default.
    let readme = project_file(&basic(), "README.md");
    assert!(readme.contains("    private_key=os.environ.get(\"ACME_PRIVATE_KEY\"),"));
    assert!(!readme.contains("password="));
    assert!(
        project_file(&header(), "tests/test_client.py")
            .contains("assert request.headers.get(\"X-API-Key\") == x_api_key")
    );
    // No auth (opencode): no credentials anywhere, nor a header check.
    let free = authed(Vec::new(), Vec::new());
    let tests = project_file(&free, "tests/test_client.py");
    assert!(!tests.contains("test_validate_headers") && !tests.contains("api_key"));
    assert!(tests.contains("client = Acme(base_url=base_url, _strict_response_validation=True)"));
    let readme = project_file(&free, "README.md");
    assert!(
        readme.contains("```python\nfrom acme import Acme\n\nclient = Acme()\n\nthing = client.things.list()\n```")
    );
    assert!(!readme.contains("python-dotenv"));
    let conftest = project_file(&free, "tests/conftest.py");
    assert!(
        conftest.contains(
            "base_url = os.environ.get(\"TEST_API_BASE_URL\", \"http://127.0.0.1:4010\")\n\n\n@pytest.fixture"
        )
    );
}

/// A bearer client whose `api_key` reads no environment variable.
fn without_read_env() -> SdkClient {
    let mut opts = settings().opts;
    opts[0].read_env = None;
    authed(opts, settings().security_schemes)
}

#[test]
fn unsets_no_variable_for_a_credential_without_read_env() {
    let tests = project_file(&without_read_env(), "tests/test_client.py");
    assert!(!tests.contains("**{,}"), "invalid Python");
    assert!(tests.contains("            with update_env(**{}):\n"));
}

#[test]
fn shows_a_placeholder_for_a_credential_without_read_env() {
    let readme = project_file(&without_read_env(), "README.md");
    assert!(!readme.contains("os.environ"), "{readme}");
    assert!(readme.contains("```python\nfrom acme import Acme\n\nclient = Acme(\n    api_key=\"My API Key\",\n)\n"));
    assert!(readme.contains("client = AsyncAcme(\n    api_key=\"My API Key\",\n)\n"));
    assert!(readme.contains("    async with AsyncAcme(\n        api_key=\"My API Key\",\n        http_client="));
    assert!(!readme.contains("python-dotenv"));
}

fn path_param(name: &str) -> SdkParam {
    SdkParam {
        required: true,
        ..param(name, name)
    }
}

/// `retrieve` at `path`, the README example by `readme.example_requests.default`.
fn probe_of(path: &str, path_params: Vec<SdkParam>, params: &serde_json::Value) -> ExampleRequest {
    let mut m = method(Some(SdkType::Object(obj_with("id"))));
    m.name = "retrieve".into();
    m.path = path.into();
    m.path_params = path_params;
    let meta = ProjectMeta {
        readme_request: Some(ReadmeRequest {
            endpoint: format!("get {path}"),
            params: params.as_object().expect("an object").clone(),
        }),
        ..placeholder_meta()
    };
    to_project_files_input(&sdk(vec![m]), &meta)
        .expect("an input")
        .retry_probe
}

fn arg_pairs(e: &ExampleRequest) -> Vec<(&str, &str)> {
    e.args.iter().map(|a| (a.name.as_str(), a.value.as_str())).collect()
}

#[test]
fn resolves_the_retry_probe_path_as_the_client_sends_it() {
    // knock: `respx_mock.get("/v1/users/user_id")` for `client.users.with_raw_response.get("user_id")`.
    let probe = probe_of(
        "/v1/users/{user_id}",
        vec![path_param("user_id")],
        &serde_json::json!({"user_id": "u"}),
    );
    assert_eq!(probe.path, "/v1/users/user_id");
    assert_eq!(arg_pairs(&probe), [("", "\"user_id\"")]);
    // Percent-encoded as `path_template` encodes a path segment.
    let odd = SdkParam {
        example: Some(serde_json::json!("a b/c@d")),
        ..path_param("user_id")
    };
    let probe = probe_of("/v1/users/{user_id}", vec![odd], &serde_json::json!({}));
    assert_eq!(probe.path, "/v1/users/a%20b%2Fc@d");
}

#[test]
fn writes_date_arguments_the_example_files_can_run_without_imports() {
    // tests/test_client.py and README.md import no `parse_date`, so a required date or datetime
    // goes as its ISO string, which the `Union[str, date]` parameter accepts.
    let mut m = method(Some(SdkType::Object(obj_with("id"))));
    m.name = "create".into();
    m.http_method = HttpVerb::Post;
    m.body_params = vec![
        SdkParam {
            required: true,
            ty: SdkType::Primitive(SdkPrimitive::Date),
            ..param("since", "since")
        },
        SdkParam {
            required: true,
            ty: SdkType::Primitive(SdkPrimitive::Datetime),
            ..param("until", "until")
        },
    ];
    let c = sdk(vec![m]);
    let input = to_project_files_input(&c, &placeholder_meta()).expect("an input");
    assert_eq!(
        arg_pairs(&input.retry_probe),
        [("since", "\"2019-12-27\""), ("until", "\"2019-12-27T18:11:19.117Z\"")]
    );
    for path in ["tests/test_client.py", "README.md"] {
        assert!(!project_file(&c, path).contains("parse_date"), "{path}");
    }
}

#[test]
fn names_the_path_arguments_of_a_retry_probe_with_several() {
    // `retrieve(self, project_id, file_id, *, ...)`: positionally, the last-first order the tests
    // use would swap them.
    let probe = probe_of(
        "/projects/{project_id}/files/{file_id}",
        vec![path_param("project_id"), path_param("file_id")],
        &serde_json::json!({"project_id": "p", "file_id": "f"}),
    );
    assert_eq!(probe.path, "/projects/project_id/files/file_id");
    assert_eq!(
        arg_pairs(&probe),
        [("file_id", "\"file_id\""), ("project_id", "\"project_id\"")]
    );
}

/// A model whose extra values are itself (`additionalProperties: {$ref: Node}` beside
/// `properties`) refers to itself as a string and is rebuilt, as through a field.
#[test]
fn defers_a_model_that_refers_to_itself_through_its_extra_values() {
    let node = SdkRef {
        name: "Node".into(),
        module: "node".into(),
    };
    let mut c = client(Vec::new(), None, "a/b");
    c.models = vec![SdkModelDecl {
        r#ref: node.clone(),
        description: String::new(),
        ty: SdkObject {
            extra: Some(Box::new(SdkType::Ref(node))),
            ..obj_with("name")
        },
        alias: None,
    }];
    let files = emit_root_files(&c, None).expect("emits");
    let get = |path: &str| files.iter().find(|(p, _)| p == path).map(|(_, t)| t.as_str());
    let module = get("src/acme/types/node.py").expect("node.py");
    assert!(
        module.contains("        __pydantic_extra__: Dict[str, \"Node\"]\n"),
        "{module}"
    );
    let init = get("src/acme/types/__init__.py").expect("types/__init__.py");
    assert!(init.contains("Node.model_rebuild("), "{init}");
}

/// A union of a titled object and an array of inline objects declares a class for the array's
/// items too, in a response and in params, named `List` after the array as ogx names the array
/// member of `InterleavedContent` (`List[ListImageContentItemTextContentItem]`).
#[test]
fn names_the_items_of_an_array_beside_a_titled_union_member() {
    let titled = SdkType::Object(SdkObject {
        hint: Some("Circle".into()),
        ..obj_with("radius")
    });
    let union = SdkType::Union {
        variants: vec![titled, SdkType::Array(Box::new(SdkType::Object(obj_with("x"))))],
        discriminator: None,
    };
    let response = SdkObject {
        fields: vec![field("shape", "shape", true, "", union.clone())],
        ..SdkObject::default()
    };
    let mut m = method(Some(SdkType::Object(response)));
    m.http_method = HttpVerb::Post;
    m.body_params = vec![SdkParam {
        ty: union,
        required: true,
        ..param("shape", "shape")
    }];
    m.body = Some(SdkType::Object(obj_with("id")));
    let c = client(vec![resource(&["things"], vec![m])], None, "a/b");
    let files = emit_root_files(&c, None).expect("emits");
    let get = |path: &str| files.iter().find(|(p, _)| p == path).map(|(_, t)| t.as_str());
    let response = get("src/acme/types/thing_list_response.py").expect("response module");
    assert!(
        response.contains(
            "ThingListResponseShape: TypeAlias = Union[ThingListResponseShapeCircle, List[ThingListResponseShapeList]]"
        ),
        "{response}"
    );
    assert!(
        response.contains("class ThingListResponseShapeList(BaseModel):\n    x: str\n"),
        "{response}"
    );
    let params = get("src/acme/types/thing_list_params.py").expect("params module");
    assert!(
        params.contains("Shape: TypeAlias = Union[ShapeCircle, Iterable[ShapeList]]"),
        "{params}"
    );
    assert!(
        params.contains("class ShapeList(TypedDict, total=False):\n    x: Required[str]\n"),
        "{params}"
    );
}

/// Header parameters reach the project files after the query and body parameters, by wire name
/// (ogx `models.list`); a method whose only keyword arguments are headers has no params module.
#[test]
fn maps_header_params_after_the_others_into_the_project_files() {
    let mut m = method(Some(SdkType::Object(obj_with("id"))));
    m.query_params = vec![param("zone", "zone")];
    m.header_params = vec![
        param("x-goog-api-key", "x_goog_api_key"),
        param("anthropic-version", "anthropic_version"),
    ];
    let mut only_headers = m.clone();
    only_headers.name = "retrieve".into();
    only_headers.query_params = Vec::new();
    let key = SdkClientOpt {
        send: Some(SdkSend {
            location: SendLocation::Query,
            name: "key".into(),
        }),
        ..settings().opts[0].clone()
    };
    let c = client(
        vec![resource(&["things"], vec![m, only_headers])],
        Some(SdkClientSettings {
            opts: vec![key],
            ..settings()
        }),
        "acme/acme-python",
    );
    let input = to_project_files_input(&c, &placeholder_meta()).expect("an input");
    let by_name = |name: &str| {
        input.resources[0]
            .methods
            .iter()
            .find(|m| m.name == name)
            .expect("the method")
    };
    let names: Vec<&str> = by_name("list").params.iter().map(|p| p.py_name.as_str()).collect();
    assert_eq!(names, ["zone", "anthropic_version", "x_goog_api_key"]);
    assert_eq!(
        by_name("list").params_module.as_deref(),
        Some("types.thing_list_params")
    );
    assert_eq!(by_name("retrieve").params.len(), 2);
    assert_eq!(by_name("retrieve").params_module, None);
}

/// A header parameter whose schema is an inline object is FER201: it would be sent as
/// `str(x_meta)`, and alone it leaves its class `XMeta` declared in no module.
#[test]
fn rejects_a_header_parameter_that_is_an_inline_object() {
    let meta = SdkParam {
        ty: SdkType::Object(obj_with("a")),
        ..param("x-meta", "x_meta")
    };
    let mut alone = method(Some(SdkType::Object(obj_with("id"))));
    alone.header_params = vec![meta.clone()];
    let c = plain_client(alone);
    let err = emit_resource_file(&c, first(&c)).expect_err("FER201");
    assert_eq!((err.code, err.pointer.as_str()), (FerCode::Fer201, "#/paths/~1t/get"));
    assert!(err.detail.contains("x-meta"), "{}", err.detail);
    let mut beside = method(Some(SdkType::Object(obj_with("id"))));
    beside.query_params = vec![param("q", "q")];
    beside.header_params = vec![meta];
    let c = plain_client(beside);
    assert_eq!(
        emit_resource_file(&c, first(&c)).map_err(|e| e.code),
        Err(FerCode::Fer201)
    );
}
