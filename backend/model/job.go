package model

type JobStatus string

const (
	JobQueued    JobStatus = "queued"
	JobRunning   JobStatus = "running"
	JobCompleted JobStatus = "completed"
	JobCancelled JobStatus = "cancelled"
	JobFailed    JobStatus = "failed"
)

type Job struct {
	ID        string    `json:"id"`
	FileName  string    `json:"fileName"`
	Status    JobStatus `json:"status"`
	Progress  int       `json:"progress"`
	Processed int       `json:"processed"`
	Total     int       `json:"total"`
	Result    string    `json:"result,omitempty"`
	Error     string    `json:"error,omitempty"`
}
