package main

import (
	"backend/service"
	"encoding/base64"
	"fmt"
	"os"
	"time"
)

func main() {
	if len(os.Args) != 2 {
		fmt.Fprintln(os.Stderr, "usage: sample-report OUTPUT.pdf")
		os.Exit(2)
	}
	report := service.GenerateSampleReport(time.Now())
	pdf, err := base64.StdEncoding.DecodeString(report.PDFBase64)
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
	if err := os.WriteFile(os.Args[1], pdf, 0o600); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}
