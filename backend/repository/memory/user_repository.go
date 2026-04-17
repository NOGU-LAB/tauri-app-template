package memory

import (
	"backend/model"
	"errors"
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
		return nil, errors.New("user not found")
	}
	return user, nil
}

func (r *InMemoryUserRepository) FindAll() ([]*model.User, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()

	users := make([]*model.User, 0, len(r.data))
	for _, u := range r.data {
		users = append(users, u)
	}
	return users, nil
}

func (r *InMemoryUserRepository) Save(user *model.User) error {
	r.mu.Lock()
	defer r.mu.Unlock()

	if user.ID == 0 {
		r.counter++
		user.ID = r.counter
	}
	r.data[user.ID] = user
	return nil
}

func (r *InMemoryUserRepository) Delete(id int) error {
	r.mu.Lock()
	defer r.mu.Unlock()

	if _, ok := r.data[id]; !ok {
		return errors.New("user not found")
	}
	delete(r.data, id)
	return nil
}
