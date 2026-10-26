// File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

package shared

import (
	"github.com/acme-test/acme-go/internal/apijson"
)

type ErrorInfo struct {
	Code    string        `json:"code"`
	Message string        `json:"message"`
	JSON    errorInfoJSON `json:"-"`
}

// errorInfoJSON contains the JSON metadata for the struct [ErrorInfo]
type errorInfoJSON struct {
	Code        apijson.Field
	Message     apijson.Field
	raw         string
	ExtraFields map[string]apijson.Field
}

func (r *ErrorInfo) UnmarshalJSON(data []byte) (err error) {
	return apijson.UnmarshalRoot(data, r)
}

func (r errorInfoJSON) RawJSON() string {
	return r.raw
}
