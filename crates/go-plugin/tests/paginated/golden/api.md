# Notes

Response Types:

- <a href="https://pkg.go.dev/github.com/acme-test/acme-go">acme</a>.<a href="https://pkg.go.dev/github.com/acme-test/acme-go#Note">Note</a>

Methods:

- <code title="get /v1/notes">client.Notes.<a href="https://pkg.go.dev/github.com/acme-test/acme-go#NoteService.List">List</a>(ctx <a href="https://pkg.go.dev/context">context</a>.<a href="https://pkg.go.dev/context#Context">Context</a>, query <a href="https://pkg.go.dev/github.com/acme-test/acme-go">acme</a>.<a href="https://pkg.go.dev/github.com/acme-test/acme-go#NoteListParams">NoteListParams</a>) (\*<a href="https://pkg.go.dev/github.com/acme-test/acme-go/packages/pagination">pagination</a>.<a href="https://pkg.go.dev/github.com/acme-test/acme-go/packages/pagination#ItemsCursor">ItemsCursor</a>[<a href="https://pkg.go.dev/github.com/acme-test/acme-go">acme</a>.<a href="https://pkg.go.dev/github.com/acme-test/acme-go#Note">Note</a>], <a href="https://pkg.go.dev/builtin#error">error</a>)</code>

# Boards

Response Types:

- <a href="https://pkg.go.dev/github.com/acme-test/acme-go">acme</a>.<a href="https://pkg.go.dev/github.com/acme-test/acme-go#BoardListNotesResponse">BoardListNotesResponse</a>

Methods:

- <code title="get /v1/boards/{board_id}/notes">client.Boards.<a href="https://pkg.go.dev/github.com/acme-test/acme-go#BoardService.ListNotes">ListNotes</a>(ctx <a href="https://pkg.go.dev/context">context</a>.<a href="https://pkg.go.dev/context#Context">Context</a>, boardID <a href="https://pkg.go.dev/builtin#string">string</a>, query <a href="https://pkg.go.dev/github.com/acme-test/acme-go">acme</a>.<a href="https://pkg.go.dev/github.com/acme-test/acme-go#BoardListNotesParams">BoardListNotesParams</a>) (\*<a href="https://pkg.go.dev/github.com/acme-test/acme-go/packages/pagination">pagination</a>.<a href="https://pkg.go.dev/github.com/acme-test/acme-go/packages/pagination#ItemsCursor">ItemsCursor</a>[<a href="https://pkg.go.dev/github.com/acme-test/acme-go">acme</a>.<a href="https://pkg.go.dev/github.com/acme-test/acme-go#BoardListNotesResponse">BoardListNotesResponse</a>], <a href="https://pkg.go.dev/builtin#error">error</a>)</code>
