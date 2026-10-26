// File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

package acme

import (
	"context"
	"net/http"
	"net/url"
	"slices"

	"github.com/acme-test/acme-go/internal/apijson"
	"github.com/acme-test/acme-go/internal/apiquery"
	"github.com/acme-test/acme-go/internal/param"
	"github.com/acme-test/acme-go/internal/requestconfig"
	"github.com/acme-test/acme-go/option"
	"github.com/acme-test/acme-go/packages/pagination"
)

// NoteService contains methods and other services that help with interacting with
// the acme API.
//
// Note, unlike clients, this service does not read variables from the environment
// automatically. You should not instantiate this service directly, and instead use
// the [NewNoteService] method instead.
type NoteService struct {
	Options []option.RequestOption
}

// NewNoteService generates a new service that applies the given options to each
// request. These options are applied after the parent client's options (if there
// is one), and before any request-specific options.
func NewNoteService(opts ...option.RequestOption) (r *NoteService) {
	r = &NoteService{}
	r.Options = opts
	return
}

// Lists the notes, newest first.
func (r *NoteService) List(ctx context.Context, query NoteListParams, opts ...option.RequestOption) (res *pagination.ItemsCursor[Note], err error) {
	var raw *http.Response
	opts = slices.Concat(r.Options, opts)
	opts = append([]option.RequestOption{option.WithResponseInto(&raw)}, opts...)
	path := "v1/notes"
	cfg, err := requestconfig.NewRequestConfig(ctx, http.MethodGet, path, query, &res, opts...)
	if err != nil {
		return nil, err
	}
	err = cfg.Execute()
	if err != nil {
		return nil, err
	}
	res.SetPageConfig(cfg, raw)
	return res, nil
}

// Lists the notes, newest first.
func (r *NoteService) ListAutoPaging(ctx context.Context, query NoteListParams, opts ...option.RequestOption) *pagination.ItemsCursorAutoPager[Note] {
	return pagination.NewItemsCursorAutoPager(r.List(ctx, query, opts...))
}

type Note struct {
	ID   string   `json:"id" api:"required"`
	Body string   `json:"body"`
	JSON noteJSON `json:"-"`
}

// noteJSON contains the JSON metadata for the struct [Note]
type noteJSON struct {
	ID          apijson.Field
	Body        apijson.Field
	raw         string
	ExtraFields map[string]apijson.Field
}

func (r *Note) UnmarshalJSON(data []byte) (err error) {
	return apijson.UnmarshalRoot(data, r)
}

func (r noteJSON) RawJSON() string {
	return r.raw
}

type NoteListParams struct {
	// The cursor to fetch entries after.
	After param.Field[string] `query:"after"`
	// The cursor to fetch entries before.
	Before   param.Field[string] `query:"before"`
	PageSize param.Field[int64]  `query:"page_size"`
}

// URLQuery serializes [NoteListParams]'s query parameters as `url.Values`.
func (r NoteListParams) URLQuery() (v url.Values) {
	return apiquery.MarshalWithSettings(r, apiquery.QuerySettings{
		ArrayFormat:  apiquery.ArrayQueryFormatComma,
		NestedFormat: apiquery.NestedQueryFormatBrackets,
	})
}
