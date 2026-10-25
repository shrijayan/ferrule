# Shared Response Types

- <a href="https://pkg.go.dev/github.com/acme-test/acme-go/shared">shared</a>.<a href="https://pkg.go.dev/github.com/acme-test/acme-go/shared#ErrorInfo">ErrorInfo</a>

# Widgets

Response Types:

- <a href="https://pkg.go.dev/github.com/acme-test/acme-go">acme</a>.<a href="https://pkg.go.dev/github.com/acme-test/acme-go#Widget">Widget</a>
- <a href="https://pkg.go.dev/github.com/acme-test/acme-go">acme</a>.<a href="https://pkg.go.dev/github.com/acme-test/acme-go#WidgetListResponse">WidgetListResponse</a>

Methods:

- <code title="post /v1/widgets">client.Widgets.<a href="https://pkg.go.dev/github.com/acme-test/acme-go#WidgetService.New">New</a>(ctx <a href="https://pkg.go.dev/context">context</a>.<a href="https://pkg.go.dev/context#Context">Context</a>, body <a href="https://pkg.go.dev/github.com/acme-test/acme-go">acme</a>.<a href="https://pkg.go.dev/github.com/acme-test/acme-go#WidgetNewParams">WidgetNewParams</a>) (\*<a href="https://pkg.go.dev/github.com/acme-test/acme-go">acme</a>.<a href="https://pkg.go.dev/github.com/acme-test/acme-go#Widget">Widget</a>, <a href="https://pkg.go.dev/builtin#error">error</a>)</code>
- <code title="get /v1/widgets/{widget_id}">client.Widgets.<a href="https://pkg.go.dev/github.com/acme-test/acme-go#WidgetService.Get">Get</a>(ctx <a href="https://pkg.go.dev/context">context</a>.<a href="https://pkg.go.dev/context#Context">Context</a>, widgetID <a href="https://pkg.go.dev/builtin#string">string</a>) (\*<a href="https://pkg.go.dev/github.com/acme-test/acme-go">acme</a>.<a href="https://pkg.go.dev/github.com/acme-test/acme-go#Widget">Widget</a>, <a href="https://pkg.go.dev/builtin#error">error</a>)</code>
- <code title="get /v1/widgets">client.Widgets.<a href="https://pkg.go.dev/github.com/acme-test/acme-go#WidgetService.List">List</a>(ctx <a href="https://pkg.go.dev/context">context</a>.<a href="https://pkg.go.dev/context#Context">Context</a>, query <a href="https://pkg.go.dev/github.com/acme-test/acme-go">acme</a>.<a href="https://pkg.go.dev/github.com/acme-test/acme-go#WidgetListParams">WidgetListParams</a>) (\*<a href="https://pkg.go.dev/github.com/acme-test/acme-go">acme</a>.<a href="https://pkg.go.dev/github.com/acme-test/acme-go#WidgetListResponse">WidgetListResponse</a>, <a href="https://pkg.go.dev/builtin#error">error</a>)</code>
- <code title="delete /v1/widgets/{widget_id}">client.Widgets.<a href="https://pkg.go.dev/github.com/acme-test/acme-go#WidgetService.Delete">Delete</a>(ctx <a href="https://pkg.go.dev/context">context</a>.<a href="https://pkg.go.dev/context#Context">Context</a>, widgetID <a href="https://pkg.go.dev/builtin#string">string</a>) <a href="https://pkg.go.dev/builtin#error">error</a></code>

## Parts

Response Types:

- <a href="https://pkg.go.dev/github.com/acme-test/acme-go">acme</a>.<a href="https://pkg.go.dev/github.com/acme-test/acme-go#WidgetPartGetResponse">WidgetPartGetResponse</a>

Methods:

- <code title="get /v1/widgets/{widget_id}/parts/{part_number}">client.Widgets.Parts.<a href="https://pkg.go.dev/github.com/acme-test/acme-go#WidgetPartService.Get">Get</a>(ctx <a href="https://pkg.go.dev/context">context</a>.<a href="https://pkg.go.dev/context#Context">Context</a>, widgetID <a href="https://pkg.go.dev/builtin#string">string</a>, partNumber <a href="https://pkg.go.dev/builtin#int64">int64</a>) (\*<a href="https://pkg.go.dev/github.com/acme-test/acme-go">acme</a>.<a href="https://pkg.go.dev/github.com/acme-test/acme-go#WidgetPartGetResponse">WidgetPartGetResponse</a>, <a href="https://pkg.go.dev/builtin#error">error</a>)</code>
