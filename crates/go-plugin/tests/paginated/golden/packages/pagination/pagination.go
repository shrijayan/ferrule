// File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

package pagination

import (
	"net/http"

	"github.com/acme-test/acme-go/internal/apijson"
	"github.com/acme-test/acme-go/internal/requestconfig"
	"github.com/acme-test/acme-go/option"
)

type ItemsCursorPageInfo struct {
	After string                  `json:"after"`
	JSON  itemsCursorPageInfoJSON `json:"-"`
}

// itemsCursorPageInfoJSON contains the JSON metadata for the struct
// [ItemsCursorPageInfo]
type itemsCursorPageInfoJSON struct {
	After       apijson.Field
	raw         string
	ExtraFields map[string]apijson.Field
}

func (r *ItemsCursorPageInfo) UnmarshalJSON(data []byte) (err error) {
	return apijson.UnmarshalRoot(data, r)
}

func (r itemsCursorPageInfoJSON) RawJSON() string {
	return r.raw
}

type ItemsCursor[T any] struct {
	Items    []T                 `json:"items"`
	PageInfo ItemsCursorPageInfo `json:"page_info"`
	JSON     itemsCursorJSON     `json:"-"`
	cfg      *requestconfig.RequestConfig
	res      *http.Response
}

// itemsCursorJSON contains the JSON metadata for the struct [ItemsCursor[T]]
type itemsCursorJSON struct {
	Items       apijson.Field
	PageInfo    apijson.Field
	raw         string
	ExtraFields map[string]apijson.Field
}

func (r *ItemsCursor[T]) UnmarshalJSON(data []byte) (err error) {
	return apijson.UnmarshalRoot(data, r)
}

func (r itemsCursorJSON) RawJSON() string {
	return r.raw
}

// GetNextPage returns the next page as defined by this pagination style. When
// there is no next page, this function will return a 'nil' for the page value, but
// will not return an error
func (r *ItemsCursor[T]) GetNextPage() (res *ItemsCursor[T], err error) {
	next := r.PageInfo.After
	if len(next) == 0 {
		return nil, nil
	}
	cfg := r.cfg.Clone(r.cfg.Context)
	err = cfg.Apply(option.WithQuery("after", next))
	if err != nil {
		return nil, err
	}
	var raw *http.Response
	cfg.ResponseInto = &raw
	cfg.ResponseBodyInto = &res
	err = cfg.Execute()
	if err != nil {
		return nil, err
	}
	res.SetPageConfig(cfg, raw)
	return res, nil
}

func (r *ItemsCursor[T]) SetPageConfig(cfg *requestconfig.RequestConfig, res *http.Response) {
	if r == nil {
		r = &ItemsCursor[T]{}
	}
	r.cfg = cfg
	r.res = res
}

type ItemsCursorAutoPager[T any] struct {
	page *ItemsCursor[T]
	cur  T
	idx  int
	run  int
	err  error
}

func NewItemsCursorAutoPager[T any](page *ItemsCursor[T], err error) *ItemsCursorAutoPager[T] {
	return &ItemsCursorAutoPager[T]{
		page: page,
		err:  err,
	}
}

func (r *ItemsCursorAutoPager[T]) Next() bool {
	if r.page == nil || len(r.page.Items) == 0 {
		return false
	}
	if r.idx >= len(r.page.Items) {
		r.idx = 0
		r.page, r.err = r.page.GetNextPage()
		if r.err != nil || r.page == nil || len(r.page.Items) == 0 {
			return false
		}
	}
	r.cur = r.page.Items[r.idx]
	r.run += 1
	r.idx += 1
	return true
}

func (r *ItemsCursorAutoPager[T]) Current() T {
	return r.cur
}

func (r *ItemsCursorAutoPager[T]) Err() error {
	return r.err
}

func (r *ItemsCursorAutoPager[T]) Index() int {
	return r.run
}
