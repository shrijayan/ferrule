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

func TestAutoPagination(t *testing.T) {
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
	iter := client.Notes.ListAutoPaging(context.TODO(), acme.NoteListParams{})
	// The mock server isn't going to give us real pagination
	for i := 0; i < 3 && iter.Next(); i++ {
		note := iter.Current()
		t.Logf("%+v\n", note.ID)
	}
	if err := iter.Err(); err != nil {
		t.Fatalf("err should be nil: %s", err.Error())
	}
}
