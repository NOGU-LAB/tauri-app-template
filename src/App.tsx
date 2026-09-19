import { useState } from "react";
import { Alert, Badge, Container, Nav, Spinner } from "react-bootstrap";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faServer } from "@fortawesome/free-solid-svg-icons";
import { DesktopDemo } from "./components/DesktopDemo";
import { PrinterDemo } from "./components/PrinterDemo";
import { UsersDemo } from "./components/UsersDemo";
import { useBackend } from "./hooks/useBackend";

type View = "desktop" | "printer" | "users";

function App() {
  const { apiBase, token, isReady, backendError } = useBackend();
  const [view, setView] = useState<View>("desktop");

  if (!isReady) {
    return (
      <Container className="d-flex justify-content-center align-items-center vh-100">
        <div className="text-center text-muted">
          {backendError ? (
            <Alert variant="danger">{backendError}</Alert>
          ) : (
            <><Spinner animation="border" className="mb-3" /><p>バックエンド起動中…</p></>
          )}
        </div>
      </Container>
    );
  }

  return (
    <Container className="app-shell py-4">
      <header className="d-flex flex-wrap justify-content-between align-items-start gap-3 mb-4">
        <div>
          <h1 className="h3 mb-1">Tauri + React + Go</h1>
          <p className="text-muted small mb-0">
            <FontAwesomeIcon icon={faServer} className="me-1" />{apiBase}
            <Badge bg="success" className="ms-2">接続済み</Badge>
          </p>
        </div>
        <Nav variant="pills" activeKey={view} onSelect={(key) => key && setView(key as View)}>
          <Nav.Item><Nav.Link eventKey="desktop">Desktop Showcase</Nav.Link></Nav.Item>
          <Nav.Item><Nav.Link eventKey="printer">Printer</Nav.Link></Nav.Item>
          <Nav.Item><Nav.Link eventKey="users">Users CRUD</Nav.Link></Nav.Item>
        </Nav>
      </header>
      {view === "desktop" && <DesktopDemo apiBase={apiBase} token={token} />}
      {view === "printer" && <PrinterDemo apiBase={apiBase} token={token} />}
      {view === "users" && <UsersDemo apiBase={apiBase} token={token} />}
    </Container>
  );
}

export default App;
