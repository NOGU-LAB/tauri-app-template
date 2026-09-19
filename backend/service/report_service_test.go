package service

import (
	"encoding/base64"
	"strings"
	"testing"
	"time"
)

func TestGenerateSampleReport(t *testing.T) {
	report := GenerateSampleReport(time.Date(2026, 9, 19, 1, 2, 3, 0, time.UTC))
	pdf, err := base64.StdEncoding.DecodeString(report.PDFBase64)
	if err != nil {
		t.Fatal(err)
	}
	if report.Size != len(pdf) || !strings.HasPrefix(string(pdf), "%PDF-1.4") || !strings.HasSuffix(string(pdf), "%%EOF\n") {
		t.Fatalf("invalid PDF output: size=%d bytes=%d", report.Size, len(pdf))
	}
	if !strings.Contains(string(pdf), "Tauri Desktop Sample Report") {
		t.Fatal("report title is missing")
	}
}
