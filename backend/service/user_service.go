package service

import (
	"backend/model"
	"backend/repository"
	"errors"
	"net/mail"
	"strings"
)

var (
	ErrInvalidName  = errors.New("name is required")
	ErrInvalidEmail = errors.New("email is invalid")
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
	name = strings.TrimSpace(name)
	email = strings.TrimSpace(email)
	if name == "" {
		return nil, ErrInvalidName
	}
	address, err := mail.ParseAddress(email)
	if err != nil || address.Address != email {
		return nil, ErrInvalidEmail
	}
	user := &model.User{Name: name, Email: email}
	if err := s.repo.Save(user); err != nil {
		return nil, err
	}
	return user, nil
}

func (s *UserService) DeleteUser(id int) error {
	return s.repo.Delete(id)
}
