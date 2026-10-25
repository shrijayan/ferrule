// File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

package acme_test

import (
	"context"
	"errors"
	"os"
	"testing"

	"github.com/acme-test/acme-go"
	"github.com/acme-test/acme-go/internal/testutil"
	"github.com/acme-test/acme-go/option"
)

func TestWidgetPartGet(t *testing.T) {
	baseURL := "http://localhost:4010"
	if envURL, ok := os.LookupEnv("TEST_API_BASE_URL"); ok {
		baseURL = envURL
	}
	if !testutil.CheckTestServer(t, baseURL) {
		return
	}
	client := acme.NewClient(
		option.WithBaseURL(baseURL),
		option.WithAPIKey("My API Key"),
	)
	_, err := client.Widgets.Parts.Get(context.TODO(), "widget_id", int64(0))
	if err != nil {
		var apierr *acme.Error
		if errors.As(err, &apierr) {
			t.Log(string(apierr.DumpRequest(true)))
		}
		t.Fatalf("err should be nil: %s", err.Error())
	}
}
