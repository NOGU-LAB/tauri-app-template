package memory

import (
	"backend/model"
	"backend/repository"
	"sort"
	"sync"
)

// InMemoryUserRepository はインメモリでユーザーを管理する（開発用・DBが決まるまで）
type InMemoryUserRepository struct {
	mu      sync.RWMutex
	data    map[int]*model.User
	counter int
}

func NewUserRepository() *InMemoryUserRepository {
	return &InMemoryUserRepository{
		data: make(map[int]*model.User),
	}
}

func (r *InMemoryUserRepository) FindByID(id int) (*model.User, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()

	user, ok := r.data[id]
	if !ok {
		return nil, repository.ErrNotFound
	}
	copy := *user
	return &copy, nil
}

func (r *InMemoryUserRepository) FindAll() ([]*model.User, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()

	users := make([]*model.User, 0, len(r.data))
	ids := make([]int, 0, len(r.data))
	for id := range r.data {
		ids = append(ids, id)
	}
	sort.Ints(ids)
	for _, id := range ids {
		copy := *r.data[id]
		users = append(users, &copy)
	}
	return users, nil
}

func (r *InMemoryUserRepository) Save(user *model.User) error {
	r.mu.Lock()
	defer r.mu.Unlock()

	if user.ID == 0 {
		r.counter++
		user.ID = r.counter
	} else if _, ok := r.data[user.ID]; !ok {
		return repository.ErrNotFound
	}
	copy := *user
	r.data[user.ID] = &copy
	return nil
}

func (r *InMemoryUserRepository) Delete(id int) error {
	r.mu.Lock()
	defer r.mu.Unlock()

	if _, ok := r.data[id]; !ok {
		return repository.ErrNotFound
	}
	delete(r.data, id)
	return nil
}
