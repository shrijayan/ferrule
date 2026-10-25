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

func TestManualPagination(t *testing.T) {
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
	page, err := client.Notes.List(context.TODO(), acme.NoteListParams{})
	if err != nil {
		t.Fatalf("err should be nil: %s", err.Error())
	}
	for _, note := range page.Items {
		t.Logf("%+v\n", note.ID)
	}
	// The mock server isn't going to give us real pagination
	page, err = page.GetNextPage()
	if err != nil {
		t.Fatalf("err should be nil: %s", err.Error())
	}
	if page != nil {
		for _, note := range page.Items {
			t.Logf("%+v\n", note.ID)
		}
	}
}
