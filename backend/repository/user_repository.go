package repository

import "backend/model"

// UserRepository はユーザーデータへのアクセスを抽象化するインターフェース
type UserRepository interface {
	FindByID(id int) (*model.User, error)
	FindAll() ([]*model.User, error)
	Save(user *model.User) error
	Delete(id int) error
}
