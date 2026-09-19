package memory

import (
	"backend/model"
	"backend/repository"
	"errors"
	"reflect"
	"testing"
)

func TestRepositoryReturnsSortedCopies(t *testing.T) {
	repo := NewUserRepository()
	first := &model.User{Name: "First", Email: "first@example.com"}
	second := &model.User{Name: "Second", Email: "second@example.com"}
	if err := repo.Save(first); err != nil {
		t.Fatal(err)
	}
	if err := repo.Save(second); err != nil {
		t.Fatal(err)
	}

	users, err := repo.FindAll()
	if err != nil {
		t.Fatal(err)
	}
	ids := []int{users[0].ID, users[1].ID}
	if !reflect.DeepEqual(ids, []int{1, 2}) {
		t.Fatalf("ids = %v, want [1 2]", ids)
	}

	users[0].Name = "mutated"
	again, err := repo.FindByID(1)
	if err != nil {
		t.Fatal(err)
	}
	if again.Name != "First" {
		t.Fatalf("repository leaked mutable pointer: %q", again.Name)
	}
}

func TestRepositoryUpdateMissingUser(t *testing.T) {
	repo := NewUserRepository()
	err := repo.Save(&model.User{ID: 999, Name: "Missing", Email: "missing@example.com"})
	if !errors.Is(err, repository.ErrNotFound) {
		t.Fatalf("error = %v, want ErrNotFound", err)
	}
}
