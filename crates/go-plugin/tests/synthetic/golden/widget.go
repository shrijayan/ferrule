// File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

package acme

import (
	"context"
	"errors"
	"fmt"
	"net/http"
	"net/url"
	"slices"
	"time"

	"github.com/acme-test/acme-go/internal/apijson"
	"github.com/acme-test/acme-go/internal/apiquery"
	"github.com/acme-test/acme-go/internal/param"
	"github.com/acme-test/acme-go/internal/requestconfig"
	"github.com/acme-test/acme-go/option"
)

// WidgetService contains methods and other services that help with interacting
// with the acme API.
//
// Note, unlike clients, this service does not read variables from the environment
// automatically. You should not instantiate this service directly, and instead use
// the [NewWidgetService] method instead.
type WidgetService struct {
	Options []option.RequestOption
	Parts   *WidgetPartService
}

// NewWidgetService generates a new service that applies the given options to each
// request. These options are applied after the parent client's options (if there
// is one), and before any request-specific options.
func NewWidgetService(opts ...option.RequestOption) (r *WidgetService) {
	r = &WidgetService{}
	r.Options = opts
	r.Parts = NewWidgetPartService(opts...)
	return
}

// Creates a widget with the given name and parts. The SKU is assigned by the
// server.
func (r *WidgetService) New(ctx context.Context, body WidgetNewParams, opts ...option.RequestOption) (res *Widget, err error) {
	opts = slices.Concat(r.Options, opts)
	path := "v1/widgets"
	err = requestconfig.ExecuteNewRequest(ctx, http.MethodPost, path, body, &res, opts...)
	return res, err
}

// Retrieve a widget
func (r *WidgetService) Get(ctx context.Context, widgetID string, opts ...option.RequestOption) (res *Widget, err error) {
	opts = slices.Concat(r.Options, opts)
	if widgetID == "" {
		err = errors.New("missing required widget_id parameter")
		return nil, err
	}
	path := fmt.Sprintf("v1/widgets/%s", widgetID)
	err = requestconfig.ExecuteNewRequest(ctx, http.MethodGet, path, nil, &res, opts...)
	return res, err
}

// List widgets
func (r *WidgetService) List(ctx context.Context, query WidgetListParams, opts ...option.RequestOption) (res *WidgetListResponse, err error) {
	opts = slices.Concat(r.Options, opts)
	path := "v1/widgets"
	err = requestconfig.ExecuteNewRequest(ctx, http.MethodGet, path, query, &res, opts...)
	return res, err
}

// Delete a widget
func (r *WidgetService) Delete(ctx context.Context, widgetID string, opts ...option.RequestOption) (err error) {
	opts = slices.Concat(r.Options, opts)
	opts = append([]option.RequestOption{option.WithHeader("Accept", "")}, opts...)
	if widgetID == "" {
		err = errors.New("missing required widget_id parameter")
		return err
	}
	path := fmt.Sprintf("v1/widgets/%s", widgetID)
	err = requestconfig.ExecuteNewRequest(ctx, http.MethodDelete, path, nil, nil, opts...)
	return err
}

// A widget in the catalogue.
type Widget struct {
	ID        string    `json:"id" api:"required" format:"uuid"`
	CreatedAt time.Time `json:"created_at" api:"required" format:"date-time"`
	Name      string    `json:"name" api:"required"`
	// Where the widget is in its life cycle.
	Status      WidgetStatus `json:"status" api:"required"`
	HomepageURL string       `json:"homepage_url"`
	Parts       []WidgetPart `json:"parts"`
	SKU         string       `json:"sku" api:"nullable"`
	WeightKg    float64      `json:"weight_kg"`
	JSON        widgetJSON   `json:"-"`
}

// widgetJSON contains the JSON metadata for the struct [Widget]
type widgetJSON struct {
	ID          apijson.Field
	CreatedAt   apijson.Field
	Name        apijson.Field
	Status      apijson.Field
	HomepageURL apijson.Field
	Parts       apijson.Field
	SKU         apijson.Field
	WeightKg    apijson.Field
	raw         string
	ExtraFields map[string]apijson.Field
}

func (r *Widget) UnmarshalJSON(data []byte) (err error) {
	return apijson.UnmarshalRoot(data, r)
}

func (r widgetJSON) RawJSON() string {
	return r.raw
}

// Where the widget is in its life cycle.
type WidgetStatus string

const (
	WidgetStatusDraft   WidgetStatus = "draft"
	WidgetStatusActive  WidgetStatus = "active"
	WidgetStatusRetired WidgetStatus = "retired"
)

func (r WidgetStatus) IsKnown() bool {
	switch r {
	case WidgetStatusDraft, WidgetStatusActive, WidgetStatusRetired:
		return true
	}
	return false
}

type WidgetPart struct {
	Label      string         `json:"label"`
	PartNumber int64          `json:"part_number"`
	JSON       widgetPartJSON `json:"-"`
}

// widgetPartJSON contains the JSON metadata for the struct [WidgetPart]
type widgetPartJSON struct {
	Label       apijson.Field
	PartNumber  apijson.Field
	raw         string
	ExtraFields map[string]apijson.Field
}

func (r *WidgetPart) UnmarshalJSON(data []byte) (err error) {
	return apijson.UnmarshalRoot(data, r)
}

func (r widgetPartJSON) RawJSON() string {
	return r.raw
}

type WidgetListResponse struct {
	Data       []Widget               `json:"data" api:"required"`
	NextCursor string                 `json:"next_cursor" api:"nullable"`
	JSON       widgetListResponseJSON `json:"-"`
}

// widgetListResponseJSON contains the JSON metadata for the struct
// [WidgetListResponse]
type widgetListResponseJSON struct {
	Data        apijson.Field
	NextCursor  apijson.Field
	raw         string
	ExtraFields map[string]apijson.Field
}

func (r *WidgetListResponse) UnmarshalJSON(data []byte) (err error) {
	return apijson.UnmarshalRoot(data, r)
}

func (r widgetListResponseJSON) RawJSON() string {
	return r.raw
}

type WidgetNewParams struct {
	// The widget's display name.
	Name       param.Field[string]                    `json:"name" api:"required"`
	Color      param.Field[WidgetNewParamsColor]      `json:"color"`
	Dimensions param.Field[WidgetNewParamsDimensions] `json:"dimensions"`
	Tags       param.Field[[]string]                  `json:"tags"`
}

func (r WidgetNewParams) MarshalJSON() (data []byte, err error) {
	return apijson.MarshalRoot(r)
}

type WidgetNewParamsColor string

const (
	WidgetNewParamsColorRed   WidgetNewParamsColor = "red"
	WidgetNewParamsColorGreen WidgetNewParamsColor = "green"
	WidgetNewParamsColorBlue  WidgetNewParamsColor = "blue"
)

func (r WidgetNewParamsColor) IsKnown() bool {
	switch r {
	case WidgetNewParamsColorRed, WidgetNewParamsColorGreen, WidgetNewParamsColorBlue:
		return true
	}
	return false
}

type WidgetNewParamsDimensions struct {
	Width  param.Field[float64] `json:"width" api:"required"`
	Height param.Field[float64] `json:"height"`
}

func (r WidgetNewParamsDimensions) MarshalJSON() (data []byte, err error) {
	return apijson.MarshalRoot(r)
}

type WidgetListParams struct {
	Colors       param.Field[[]string]  `query:"colors"`
	CreatedAfter param.Field[time.Time] `query:"created_after" format:"date-time"`
	Limit        param.Field[int64]     `query:"limit"`
}

// URLQuery serializes [WidgetListParams]'s query parameters as `url.Values`.
func (r WidgetListParams) URLQuery() (v url.Values) {
	return apiquery.MarshalWithSettings(r, apiquery.QuerySettings{
		ArrayFormat:  apiquery.ArrayQueryFormatComma,
		NestedFormat: apiquery.NestedQueryFormatBrackets,
	})
}
