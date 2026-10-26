package __ferrule_pkg__

import (
//ferrule:if module_before_io
	"__ferrule_module__/packages/param"
	"io"
	"time"
//ferrule:else
	"io"
	"time"
	"__ferrule_module__/packages/param"
//ferrule:endif
)

func String(s string) param.Opt[string]     { return param.NewOpt(s) }
func Int(i int64) param.Opt[int64]          { return param.NewOpt(i) }
func Bool(b bool) param.Opt[bool]           { return param.NewOpt(b) }
func Float(f float64) param.Opt[float64]    { return param.NewOpt(f) }
func Time(t time.Time) param.Opt[time.Time] { return param.NewOpt(t) }

func Opt[T comparable](v T) param.Opt[T] { return param.NewOpt(v) }
func Ptr[T any](v T) *T                  { return &v }

func IntPtr(v int64) *int64          { return &v }
func BoolPtr(v bool) *bool           { return &v }
func FloatPtr(v float64) *float64    { return &v }
func StringPtr(v string) *string     { return &v }
func TimePtr(v time.Time) *time.Time { return &v }

//ferrule:if new_file
func NewFile(rdr io.Reader, filename string, contentType string) file {
//ferrule:else
func File(rdr io.Reader, filename string, contentType string) file {
//ferrule:endif
	return file{rdr, filename, contentType}
}

type file struct {
	io.Reader
	name        string
	contentType string
}

func (f file) Filename() string {
	if f.name != "" {
		return f.name
	} else if named, ok := f.Reader.(interface{ Name() string }); ok {
		return named.Name()
	}
	return ""
}

func (f file) ContentType() string {
	return f.contentType
}
