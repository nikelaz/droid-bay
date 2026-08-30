package helpers

import (
	"fmt"
	"strings"
)

func SplitArgs(s string) ([]string, error) {
	var args []string
	var cur strings.Builder
	var inSingle, inDouble, escaped bool
	for _, r := range s {
		if escaped {
			cur.WriteRune(r)
			escaped = false
			continue
		}
		switch {
		case inSingle:
			if r == '\'' {
				inSingle = false
			} else {
				cur.WriteRune(r)
			}
		case inDouble:
			switch r {
			case '"':
				inDouble = false
			case '\\':
				escaped = true
			default:
				cur.WriteRune(r)
			}
		case r == '\'':
			inSingle = true
		case r == '"':
			inDouble = true
		case r == '\\':
			escaped = true
		case r == ' ' || r == '\t' || r == '\r' || r == '\n':
			if cur.Len() > 0 {
				args = append(args, cur.String())
				cur.Reset()
			}
		default:
			cur.WriteRune(r)
		}
	}
	if escaped {
		return nil, fmt.Errorf("trailing backslash in %q", s)
	}
	if inSingle || inDouble {
		return nil, fmt.Errorf("unterminated quote in %q", s)
	}
	if cur.Len() > 0 {
		args = append(args, cur.String())
	}
	return args, nil
}