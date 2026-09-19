package service

import (
	"context"
	"crypto/rand"
	"encoding/csv"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"path/filepath"
	"strings"
	"sync"
	"time"

	"backend/model"
)

const maxJobContentSize = 5 << 20
const jobRetention = 10 * time.Minute

var (
	ErrInvalidJobFile    = errors.New("CSVまたはJSONファイルを選択してください")
	ErrInvalidJobContent = errors.New("CSVまたはJSONの内容が不正です")
	ErrJobTooLarge       = errors.New("ファイルサイズは5MB以下にしてください")
	ErrJobNotFound       = errors.New("job not found")
	ErrJobNotRunning     = errors.New("job is not running")
)

type jobEntry struct {
	job    model.Job
	cancel context.CancelFunc
	result string
}

type JobService struct {
	mu        sync.RWMutex
	jobs      map[string]*jobEntry
	stepDelay time.Duration
}

func NewJobService(stepDelay time.Duration) *JobService {
	return &JobService{jobs: make(map[string]*jobEntry), stepDelay: stepDelay}
}

func (s *JobService) Start(fileName, content string) (model.Job, error) {
	fileName = filepath.Base(strings.TrimSpace(fileName))
	if len(content) > maxJobContentSize {
		return model.Job{}, ErrJobTooLarge
	}

	result, itemCount, err := prepareJobResult(fileName, content)
	if err != nil {
		return model.Job{}, err
	}

	id, err := newJobID()
	if err != nil {
		return model.Job{}, fmt.Errorf("job id: %w", err)
	}
	ctx, cancel := context.WithCancel(context.Background())
	total := itemCount
	if total < 20 {
		total = 20
	}
	job := model.Job{
		ID: id, FileName: fileName, Status: model.JobQueued, Total: total,
	}
	entry := &jobEntry{job: job, cancel: cancel, result: result}
	s.mu.Lock()
	s.jobs[id] = entry
	s.mu.Unlock()

	go s.run(ctx, id)
	return job, nil
}

func (s *JobService) Get(id string) (model.Job, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()
	entry, ok := s.jobs[id]
	if !ok {
		return model.Job{}, ErrJobNotFound
	}
	return entry.job, nil
}

func (s *JobService) Cancel(id string) (model.Job, error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	entry, ok := s.jobs[id]
	if !ok {
		return model.Job{}, ErrJobNotFound
	}
	if entry.job.Status != model.JobQueued && entry.job.Status != model.JobRunning {
		return model.Job{}, ErrJobNotRunning
	}
	entry.cancel()
	entry.job.Status = model.JobCancelled
	time.AfterFunc(jobRetention, func() { s.deleteTerminal(id) })
	return entry.job, nil
}

func (s *JobService) run(ctx context.Context, id string) {
	if !s.markRunning(id) {
		return
	}

	for {
		entry, err := s.Get(id)
		if err != nil || entry.Processed >= entry.Total {
			break
		}
		select {
		case <-ctx.Done():
			s.update(id, func(job *model.Job) { job.Status = model.JobCancelled })
			return
		case <-time.After(s.stepDelay):
			s.update(id, func(job *model.Job) {
				if job.Status != model.JobRunning {
					return
				}
				job.Processed++
				job.Progress = job.Processed * 100 / job.Total
			})
		}
	}

	s.mu.Lock()
	if entry, ok := s.jobs[id]; ok && entry.job.Status == model.JobRunning {
		entry.job.Status = model.JobCompleted
		entry.job.Progress = 100
		entry.job.Result = entry.result
	}
	s.mu.Unlock()
	time.AfterFunc(jobRetention, func() { s.deleteTerminal(id) })
}

func (s *JobService) markRunning(id string) bool {
	s.mu.Lock()
	defer s.mu.Unlock()
	entry, ok := s.jobs[id]
	if !ok || entry.job.Status != model.JobQueued {
		return false
	}
	entry.job.Status = model.JobRunning
	return true
}

func (s *JobService) deleteTerminal(id string) {
	s.mu.Lock()
	defer s.mu.Unlock()
	if entry, ok := s.jobs[id]; ok && entry.job.Status != model.JobQueued && entry.job.Status != model.JobRunning {
		delete(s.jobs, id)
	}
}

func (s *JobService) update(id string, update func(*model.Job)) {
	s.mu.Lock()
	defer s.mu.Unlock()
	if entry, ok := s.jobs[id]; ok {
		update(&entry.job)
	}
}

func prepareJobResult(fileName, content string) (string, int, error) {
	ext := strings.ToLower(filepath.Ext(fileName))
	report := map[string]any{
		"sourceFile":  fileName,
		"processedAt": time.Now().UTC().Format(time.RFC3339),
	}

	switch ext {
	case ".csv":
		rows, err := csv.NewReader(strings.NewReader(content)).ReadAll()
		if err != nil {
			return "", 0, fmt.Errorf("%w: CSVを解析できません: %v", ErrInvalidJobContent, err)
		}
		if len(rows) == 0 {
			return "", 0, fmt.Errorf("%w: CSVにデータがありません", ErrInvalidJobContent)
		}
		report["format"] = "csv"
		report["itemCount"] = len(rows)
		report["rows"] = rows
		encoded, err := json.MarshalIndent(report, "", "  ")
		return string(encoded), len(rows), err
	case ".json":
		var data any
		decoder := json.NewDecoder(strings.NewReader(content))
		decoder.UseNumber()
		if err := decoder.Decode(&data); err != nil {
			return "", 0, fmt.Errorf("%w: JSONを解析できません: %v", ErrInvalidJobContent, err)
		}
		if err := decoder.Decode(&struct{}{}); err != io.EOF {
			return "", 0, fmt.Errorf("%w: JSONには1つの値だけを含めてください", ErrInvalidJobContent)
		}
		count := 1
		if values, ok := data.([]any); ok {
			count = len(values)
		}
		report["format"] = "json"
		report["itemCount"] = count
		report["data"] = data
		encoded, err := json.MarshalIndent(report, "", "  ")
		return string(encoded), count, err
	default:
		return "", 0, ErrInvalidJobFile
	}
}

func newJobID() (string, error) {
	bytes := make([]byte, 12)
	if _, err := rand.Read(bytes); err != nil {
		return "", err
	}
	return hex.EncodeToString(bytes), nil
}
