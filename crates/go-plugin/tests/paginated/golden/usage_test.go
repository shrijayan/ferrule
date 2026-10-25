// File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

package acme_test

import (
	"context"
	"os"
	"testing"

	"github.com/acme-test/acme-go"
	"github.com/acme-test/acme-go/internal/testutil"
	"github.com/acme-test/acme-go/option"
)

func TestUsage(t *testing.T) {
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
	page, err := client.Notes.List(context.TODO(), acme.NoteListParams{
		After:    acme.F("after"),
		Before:   acme.F("before"),
		PageSize: acme.F(int64(0)),
	})
	if err != nil {
		t.Error(err)
		return
	}
	t.Logf("%+v\n", page)
}
