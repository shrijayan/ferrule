// File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

package acme

import (
	"context"
	"errors"
	"fmt"
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

// BoardService contains methods and other services that help with interacting with
// the acme API.
//
// Note, unlike clients, this service does not read variables from the environment
// automatically. You should not instantiate this service directly, and instead use
// the [NewBoardService] method instead.
type BoardService struct {
	Options []option.RequestOption
}

// NewBoardService generates a new service that applies the given options to each
// request. These options are applied after the parent client's options (if there
// is one), and before any request-specific options.
func NewBoardService(opts ...option.RequestOption) (r *BoardService) {
	r = &BoardService{}
	r.Options = opts
	return
}

// List the notes of a board
func (r *BoardService) ListNotes(ctx context.Context, boardID string, query BoardListNotesParams, opts ...option.RequestOption) (res *pagination.ItemsCursor[BoardListNotesResponse], err error) {
	var raw *http.Response
	opts = slices.Concat(r.Options, opts)
	opts = append([]option.RequestOption{option.WithResponseInto(&raw)}, opts...)
	if boardID == "" {
		err = errors.New("missing required board_id parameter")
		return nil, err
	}
	path := fmt.Sprintf("v1/boards/%s/notes", boardID)
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

// List the notes of a board
func (r *BoardService) ListNotesAutoPaging(ctx context.Context, boardID string, query BoardListNotesParams, opts ...option.RequestOption) *pagination.ItemsCursorAutoPager[BoardListNotesResponse] {
	return pagination.NewItemsCursorAutoPager(r.ListNotes(ctx, boardID, query, opts...))
}

type BoardListNotesResponse struct {
	ID     string                     `json:"id" api:"required"`
	Pinned bool                       `json:"pinned"`
	JSON   boardListNotesResponseJSON `json:"-"`
}

// boardListNotesResponseJSON contains the JSON metadata for the struct
// [BoardListNotesResponse]
type boardListNotesResponseJSON struct {
	ID          apijson.Field
	Pinned      apijson.Field
	raw         string
	ExtraFields map[string]apijson.Field
}

func (r *BoardListNotesResponse) UnmarshalJSON(data []byte) (err error) {
	return apijson.UnmarshalRoot(data, r)
}

func (r boardListNotesResponseJSON) RawJSON() string {
	return r.raw
}

type BoardListNotesParams struct {
	After    param.Field[string] `query:"after"`
	Before   param.Field[string] `query:"before"`
	PageSize param.Field[int64]  `query:"page_size"`
}

// URLQuery serializes [BoardListNotesParams]'s query parameters as `url.Values`.
func (r BoardListNotesParams) URLQuery() (v url.Values) {
	return apiquery.MarshalWithSettings(r, apiquery.QuerySettings{
		ArrayFormat:  apiquery.ArrayQueryFormatComma,
		NestedFormat: apiquery.NestedQueryFormatBrackets,
	})
}
