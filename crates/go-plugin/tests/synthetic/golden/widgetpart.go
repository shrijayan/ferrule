// File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

package acme

import (
	"context"
	"errors"
	"fmt"
	"net/http"
	"slices"

	"github.com/acme-test/acme-go/internal/apijson"
	"github.com/acme-test/acme-go/internal/requestconfig"
	"github.com/acme-test/acme-go/option"
	"github.com/acme-test/acme-go/shared"
)

// WidgetPartService contains methods and other services that help with interacting
// with the acme API.
//
// Note, unlike clients, this service does not read variables from the environment
// automatically. You should not instantiate this service directly, and instead use
// the [NewWidgetPartService] method instead.
type WidgetPartService struct {
	Options []option.RequestOption
}

// NewWidgetPartService generates a new service that applies the given options to
// each request. These options are applied after the parent client's options (if
// there is one), and before any request-specific options.
func NewWidgetPartService(opts ...option.RequestOption) (r *WidgetPartService) {
	r = &WidgetPartService{}
	r.Options = opts
	return
}

// Retrieve one part of a widget
func (r *WidgetPartService) Get(ctx context.Context, widgetID string, partNumber int64, opts ...option.RequestOption) (res *WidgetPartGetResponse, err error) {
	opts = slices.Concat(r.Options, opts)
	if widgetID == "" {
		err = errors.New("missing required widgetID parameter")
		return nil, err
	}
	path := fmt.Sprintf("v1/widgets/%s/parts/%v", widgetID, partNumber)
	err = requestconfig.ExecuteNewRequest(ctx, http.MethodGet, path, nil, &res, opts...)
	return res, err
}

type WidgetPartGetResponse struct {
	ID string `json:"id" api:"required"`
	// The part's stock keeping unit.
	SKU   string                    `json:"sku" api:"required"`
	Error shared.ErrorInfo          `json:"error"`
	JSON  widgetPartGetResponseJSON `json:"-"`
}

// widgetPartGetResponseJSON contains the JSON metadata for the struct
// [WidgetPartGetResponse]
type widgetPartGetResponseJSON struct {
	ID          apijson.Field
	SKU         apijson.Field
	Error       apijson.Field
	raw         string
	ExtraFields map[string]apijson.Field
}

func (r *WidgetPartGetResponse) UnmarshalJSON(data []byte) (err error) {
	return apijson.UnmarshalRoot(data, r)
}

func (r widgetPartGetResponseJSON) RawJSON() string {
	return r.raw
}
