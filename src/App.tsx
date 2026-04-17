import { useState } from "react";
import { Container, Card, Form, Button, ListGroup, Badge, Spinner, Alert } from "react-bootstrap";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faUserPlus, faTrash, faUsers, faServer } from "@fortawesome/free-solid-svg-icons";
import { useBackend } from "./hooks/useBackend";

type User = { id: number; name: string; email: string };

function App() {
  const { apiBase, isReady } = useBackend();
  const [users, setUsers] = useState<User[]>([]);
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [error, setError] = useState("");

  async function fetchUsers() {
    try {
      const res = await fetch(`${apiBase}/api/users`);
      const data = await res.json();
      setUsers(data ?? []);
      setError("");
    } catch {
      setError("ユーザー一覧の取得に失敗しました");
    }
  }

  async function createUser(e: React.FormEvent) {
    e.preventDefault();
    try {
      await fetch(`${apiBase}/api/users`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name, email }),
      });
      setName("");
      setEmail("");
      setError("");
      fetchUsers();
    } catch {
      setError("ユーザーの追加に失敗しました");
    }
  }

  async function deleteUser(id: number) {
    try {
      await fetch(`${apiBase}/api/users/${id}`, { method: "DELETE" });
      fetchUsers();
    } catch {
      setError("削除に失敗しました");
    }
  }

  if (!isReady) {
    return (
      <Container className="d-flex justify-content-center align-items-center vh-100">
        <div className="text-center text-muted">
          <Spinner animation="border" className="mb-3" />
          <p>バックエンド起動中...</p>
        </div>
      </Container>
    );
  }

  return (
    <Container className="py-4" style={{ maxWidth: 640 }}>
      <h1 className="h4 mb-1">Tauri + React + Go</h1>
      <p className="text-muted small mb-4">
        <FontAwesomeIcon icon={faServer} className="me-1" />
        {apiBase}
      </p>

      {error && <Alert variant="danger" dismissible onClose={() => setError("")}>{error}</Alert>}

      <Card className="mb-4 shadow-sm">
        <Card.Body>
          <Card.Title className="h6">
            <FontAwesomeIcon icon={faUserPlus} className="me-2 text-primary" />
            ユーザーを追加
          </Card.Title>
          <Form onSubmit={createUser} className="d-flex gap-2 mt-3">
            <Form.Control
              placeholder="名前"
              value={name}
              onChange={(e) => setName(e.target.value)}
              required
            />
            <Form.Control
              placeholder="メールアドレス"
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              required
            />
            <Button type="submit" variant="primary" style={{ whiteSpace: "nowrap" }}>
              追加
            </Button>
          </Form>
        </Card.Body>
      </Card>

      <div className="d-flex align-items-center justify-content-between mb-2">
        <h2 className="h6 mb-0">
          <FontAwesomeIcon icon={faUsers} className="me-2 text-secondary" />
          ユーザー一覧
          <Badge bg="secondary" className="ms-2">{users.length}</Badge>
        </h2>
        <Button variant="outline-secondary" size="sm" onClick={fetchUsers}>
          更新
        </Button>
      </div>

      {users.length === 0 ? (
        <p className="text-muted text-center py-3">ユーザーがいません</p>
      ) : (
        <ListGroup>
          {users.map((u) => (
            <ListGroup.Item key={u.id} className="d-flex justify-content-between align-items-center">
              <div>
                <strong>{u.name}</strong>
                <span className="text-muted ms-2 small">{u.email}</span>
              </div>
              <Button
                variant="outline-danger"
                size="sm"
                onClick={() => deleteUser(u.id)}
              >
                <FontAwesomeIcon icon={faTrash} />
              </Button>
            </ListGroup.Item>
          ))}
        </ListGroup>
      )}
    </Container>
  );
}

export default App;
