package service

import (
	"errors"
	"testing"
	"time"

	"backend/model"
)

func TestJobCompletesAndBuildsReport(t *testing.T) {
	jobs := NewJobService(time.Millisecond)
	job, err := jobs.Start("sample.csv", "name,email\nAlice,alice@example.com\n")
	if err != nil {
		t.Fatal(err)
	}
	deadline := time.Now().Add(time.Second)
	for time.Now().Before(deadline) {
		job, err = jobs.Get(job.ID)
		if err != nil {
			t.Fatal(err)
		}
		if job.Status == model.JobCompleted {
			if job.Progress != 100 || job.Result == "" {
				t.Fatalf("unexpected completed job: %+v", job)
			}
			return
		}
		time.Sleep(time.Millisecond)
	}
	t.Fatal("job did not complete")
}

func TestJobCanBeCancelled(t *testing.T) {
	jobs := NewJobService(50 * time.Millisecond)
	job, err := jobs.Start("sample.json", `[{"id":1}]`)
	if err != nil {
		t.Fatal(err)
	}
	job, err = jobs.Cancel(job.ID)
	if err != nil {
		t.Fatal(err)
	}
	if job.Status != model.JobCancelled {
		t.Fatalf("status = %s", job.Status)
	}
	time.Sleep(10 * time.Millisecond)
	job, err = jobs.Get(job.ID)
	if err != nil {
		t.Fatal(err)
	}
	if job.Status != model.JobCancelled {
		t.Fatalf("status changed after cancellation: %s", job.Status)
	}
}

func TestJobRejectsUnsupportedFile(t *testing.T) {
	jobs := NewJobService(time.Millisecond)
	_, err := jobs.Start("sample.txt", "hello")
	if !errors.Is(err, ErrInvalidJobFile) {
		t.Fatalf("err = %v", err)
	}
}

func TestJobRejectsMalformedContentWithSentinel(t *testing.T) {
	jobs := NewJobService(time.Millisecond)
	_, err := jobs.Start("sample.json", "not-json")
	if !errors.Is(err, ErrInvalidJobContent) {
		t.Fatalf("err = %v", err)
	}
}
