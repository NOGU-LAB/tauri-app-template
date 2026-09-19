package sqlite_test

import (
	"backend/infra"
	"backend/model"
	"backend/repository"
	"backend/repository/sqlite"
	"errors"
	"path/filepath"
	"testing"
)

func TestRepositoryCRUD(t *testing.T) {
	db, err := infra.NewSQLite(filepath.Join(t.TempDir(), "test.db"))
	if err != nil {
		t.Fatal(err)
	}
	defer db.Close()
	repo := sqlite.NewUserRepository(db)

	user := &model.User{Name: "Alice", Email: "alice@example.com"}
	if err := repo.Save(user); err != nil {
		t.Fatal(err)
	}
	if user.ID == 0 {
		t.Fatal("insert did not assign an ID")
	}

	user.Name = "Updated"
	if err := repo.Save(user); err != nil {
		t.Fatal(err)
	}
	found, err := repo.FindByID(user.ID)
	if err != nil || found.Name != "Updated" {
		t.Fatalf("unexpected updated user: %#v, err=%v", found, err)
	}

	if err := repo.Delete(user.ID); err != nil {
		t.Fatal(err)
	}
	if _, err := repo.FindByID(user.ID); !errors.Is(err, repository.ErrNotFound) {
		t.Fatalf("find error = %v, want ErrNotFound", err)
	}
}

func TestRepositoryUpdateMissingUser(t *testing.T) {
	db, err := infra.NewSQLite(filepath.Join(t.TempDir(), "test.db"))
	if err != nil {
		t.Fatal(err)
	}
	defer db.Close()
	repo := sqlite.NewUserRepository(db)

	err = repo.Save(&model.User{ID: 999, Name: "Missing", Email: "missing@example.com"})
	if !errors.Is(err, repository.ErrNotFound) {
		t.Fatalf("error = %v, want ErrNotFound", err)
	}
}
