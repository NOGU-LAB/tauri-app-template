package sqlite

import (
	"backend/model"
	"database/sql"
	"errors"
)

type UserRepository struct {
	db *sql.DB
}

func NewUserRepository(db *sql.DB) *UserRepository {
	return &UserRepository{db: db}
}

func (r *UserRepository) FindByID(id int) (*model.User, error) {
	row := r.db.QueryRow(`SELECT id, name, email FROM users WHERE id = ?`, id)
	u := &model.User{}
	if err := row.Scan(&u.ID, &u.Name, &u.Email); err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil, errors.New("user not found")
		}
		return nil, err
	}
	return u, nil
}

func (r *UserRepository) FindAll() ([]*model.User, error) {
	rows, err := r.db.Query(`SELECT id, name, email FROM users ORDER BY id`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var users []*model.User
	for rows.Next() {
		u := &model.User{}
		if err := rows.Scan(&u.ID, &u.Name, &u.Email); err != nil {
			return nil, err
		}
		users = append(users, u)
	}
	return users, rows.Err()
}

func (r *UserRepository) Save(user *model.User) error {
	if user.ID == 0 {
		res, err := r.db.Exec(`INSERT INTO users (name, email) VALUES (?, ?)`, user.Name, user.Email)
		if err != nil {
			return err
		}
		id, _ := res.LastInsertId()
		user.ID = int(id)
		return nil
	}
	_, err := r.db.Exec(`UPDATE users SET name = ?, email = ? WHERE id = ?`, user.Name, user.Email, user.ID)
	return err
}

func (r *UserRepository) Delete(id int) error {
	res, err := r.db.Exec(`DELETE FROM users WHERE id = ?`, id)
	if err != nil {
		return err
	}
	n, _ := res.RowsAffected()
	if n == 0 {
		return errors.New("user not found")
	}
	return nil
}
