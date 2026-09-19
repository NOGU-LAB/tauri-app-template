import { useEffect, useState } from "react";
import { Alert, Badge, Button, Card, Form, ListGroup } from "react-bootstrap";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faTrash, faUserPlus, faUsers } from "@fortawesome/free-solid-svg-icons";
import { requestJSON } from "../api";

type User = { id: number; name: string; email: string };
type Props = { apiBase: string; token: string };

export function UsersDemo({ apiBase, token }: Props) {
  const [users, setUsers] = useState<User[]>([]);
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [error, setError] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [deletingId, setDeletingId] = useState<number | null>(null);

  async function fetchUsers() {
    setIsLoading(true);
    try {
      const data = await requestJSON<User[]>(apiBase, token, "/api/users");
      setUsers(Array.isArray(data) ? data : []);
      setError("");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "ユーザー一覧の取得に失敗しました");
    } finally { setIsLoading(false); }
  }

  useEffect(() => { void fetchUsers(); }, [apiBase, token]);

  async function createUser(event: React.FormEvent) {
    event.preventDefault();
    setIsSubmitting(true);
    try {
      await requestJSON<User>(apiBase, token, "/api/users", { method: "POST", body: JSON.stringify({ name, email }) });
      setName(""); setEmail(""); setError(""); await fetchUsers();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "ユーザーの追加に失敗しました");
    } finally { setIsSubmitting(false); }
  }

  async function deleteUser(id: number) {
    setDeletingId(id);
    try {
      await requestJSON<void>(apiBase, token, `/api/users/${id}`, { method: "DELETE" });
      await fetchUsers();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "削除に失敗しました");
    } finally { setDeletingId(null); }
  }

  return <div>
    {error && <Alert variant="danger" dismissible onClose={() => setError("")}>{error}</Alert>}
    <Card className="mb-4 shadow-sm"><Card.Body>
      <Card.Title className="h6"><FontAwesomeIcon icon={faUserPlus} className="me-2 text-primary" />ユーザーを追加</Card.Title>
      <Form onSubmit={createUser} className="d-flex gap-2 mt-3">
        <Form.Control placeholder="名前" value={name} onChange={(e) => setName(e.target.value)} required />
        <Form.Control placeholder="メールアドレス" type="email" value={email} onChange={(e) => setEmail(e.target.value)} required />
        <Button disabled={isSubmitting} type="submit" style={{ whiteSpace: "nowrap" }}>{isSubmitting ? "追加中…" : "追加"}</Button>
      </Form>
    </Card.Body></Card>
    <div className="d-flex align-items-center justify-content-between mb-2">
      <h2 className="h6 mb-0"><FontAwesomeIcon icon={faUsers} className="me-2 text-secondary" />ユーザー一覧 <Badge bg="secondary">{users.length}</Badge></h2>
      <Button disabled={isLoading} variant="outline-secondary" size="sm" onClick={fetchUsers}>{isLoading ? "更新中…" : "更新"}</Button>
    </div>
    {users.length === 0 ? <p className="text-muted text-center py-3">ユーザーがいません</p> : <ListGroup>{users.map((user) =>
      <ListGroup.Item key={user.id} className="d-flex justify-content-between align-items-center">
        <div><strong>{user.name}</strong><span className="text-muted ms-2 small">{user.email}</span></div>
        <Button variant="outline-danger" size="sm" aria-label={`${user.name}を削除`} disabled={deletingId === user.id} onClick={() => deleteUser(user.id)}><FontAwesomeIcon icon={faTrash} /></Button>
      </ListGroup.Item>)}</ListGroup>}
  </div>;
}
