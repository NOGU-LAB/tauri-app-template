package repository

import (
	"backend/model"
	"errors"
)

var ErrNotFound = errors.New("user not found")

// UserRepository はユーザーデータへのアクセスを抽象化するインターフェース
type UserRepository interface {
	FindByID(id int) (*model.User, error)
	FindAll() ([]*model.User, error)
	Save(user *model.User) error
	Delete(id int) error
}
