package service

import (
	"backend/model"
	"backend/repository"
)

type UserService struct {
	repo repository.UserRepository
}

func NewUserService(repo repository.UserRepository) *UserService {
	return &UserService{repo: repo}
}

func (s *UserService) GetUser(id int) (*model.User, error) {
	return s.repo.FindByID(id)
}

func (s *UserService) GetAllUsers() ([]*model.User, error) {
	return s.repo.FindAll()
}

func (s *UserService) CreateUser(name, email string) (*model.User, error) {
	user := &model.User{Name: name, Email: email}
	if err := s.repo.Save(user); err != nil {
		return nil, err
	}
	return user, nil
}

func (s *UserService) DeleteUser(id int) error {
	return s.repo.Delete(id)
}
