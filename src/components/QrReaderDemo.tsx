import { useEffect, useMemo, useState } from "react";
import { Badge, Button, Card, Col, Form, Row, Spinner, Table } from "react-bootstrap";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faArrowsRotate, faPlug, faQrcode, faTrash } from "@fortawesome/free-solid-svg-icons";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

type QrReaderInfo = {
  portName: string;
  product?: string;
  manufacturer?: string;
  serialNumber?: string;
  vendorId?: number;
  productId?: number;
  isLikelyScanner: boolean;
};

type ReaderState = "disconnected" | "connecting" | "connected" | "error";

type QrReaderStatus = {
  state: ReaderState;
  portName?: string;
  baudRate?: number;
  error?: string;
};

type QrScan = {
  value: string;
  portName: string;
  scannedAt: number;
};

const emptyStatus: QrReaderStatus = { state: "disconnected" };

const stateLabel: Record<ReaderState, string> = {
  disconnected: "未接続",
  connecting: "接続中",
  connected: "待受中",
  error: "エラー",
};

export function QrReaderDemo() {
  const [readers, setReaders] = useState<QrReaderInfo[]>([]);
  const [selectedPort, setSelectedPort] = useState("");
  const [baudRate, setBaudRate] = useState(9600);
  const [status, setStatus] = useState<QrReaderStatus>(emptyStatus);
  const [scans, setScans] = useState<QrScan[]>([]);
  const [loading, setLoading] = useState(true);
  const [actionError, setActionError] = useState("");

  async function refreshReaders() {
    setLoading(true);
    setActionError("");
    try {
      const values = await invoke<QrReaderInfo[]>("list_qr_readers");
      setReaders(values);
      setSelectedPort((current) => {
        if (values.some((reader) => reader.portName === current)) return current;
        return values.find((reader) => reader.isLikelyScanner)?.portName ?? values[0]?.portName ?? "";
      });
    } catch (error) {
      setActionError(String(error));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void refreshReaders();
    void invoke<QrReaderStatus>("get_qr_reader_status").then(setStatus).catch((error) => setActionError(String(error)));
    const statusListener = listen<QrReaderStatus>("qr-reader-status", ({ payload }) => setStatus(payload));
    const scanListener = listen<QrScan>("qr-code-scanned", ({ payload }) => {
      setScans((current) => [payload, ...current].slice(0, 100));
    });
    return () => {
      void statusListener.then((unlisten) => unlisten());
      void scanListener.then((unlisten) => unlisten());
    };
  }, []);

  async function connect() {
    if (!selectedPort) return;
    setActionError("");
    try {
      setStatus(await invoke<QrReaderStatus>("start_qr_reader", { portName: selectedPort, baudRate }));
    } catch (error) {
      setActionError(String(error));
    }
  }

  async function disconnect() {
    setActionError("");
    try {
      setStatus(await invoke<QrReaderStatus>("stop_qr_reader"));
    } catch (error) {
      setActionError(String(error));
    }
  }

  const selected = readers.find((reader) => reader.portName === selectedPort);
  const busy = status.state === "connecting" || status.state === "connected";
  const badgeVariant = status.state === "connected" ? "success" : status.state === "error" ? "danger" : status.state === "connecting" ? "warning" : "secondary";
  const deviceId = useMemo(() => {
    if (selected?.vendorId === undefined || selected.productId === undefined) return "";
    return `VID ${selected.vendorId.toString(16).padStart(4, "0").toUpperCase()} / PID ${selected.productId.toString(16).padStart(4, "0").toUpperCase()}`;
  }, [selected]);

  return (
    <Row className="g-4">
      <Col lg={5}>
        <Card className="h-100">
          <Card.Body>
            <div className="d-flex justify-content-between align-items-start gap-3 mb-3">
              <div>
                <Card.Title className="mb-1"><FontAwesomeIcon icon={faQrcode} className="me-2" />USB QRリーダー</Card.Title>
                <Card.Text className="small text-muted mb-0">USBシリアルをRustで受信し、Reactへ即時通知します。</Card.Text>
              </div>
              <Badge bg={badgeVariant}>{stateLabel[status.state]}</Badge>
            </div>

            <Form.Group className="mb-3">
              <div className="d-flex justify-content-between align-items-center mb-1">
                <Form.Label className="mb-0">シリアルポート</Form.Label>
                <Button size="sm" variant="link" className="p-0" onClick={refreshReaders} disabled={loading || busy}>
                  {loading ? <Spinner size="sm" /> : <FontAwesomeIcon icon={faArrowsRotate} />} 再読込
                </Button>
              </div>
              <Form.Select value={selectedPort} onChange={(event) => setSelectedPort(event.target.value)} disabled={busy || loading}>
                {readers.length === 0 && <option value="">利用可能なポートがありません</option>}
                {readers.map((reader) => (
                  <option key={reader.portName} value={reader.portName}>
                    {reader.isLikelyScanner ? "★ " : ""}{reader.product ?? reader.portName} ({reader.portName})
                  </option>
                ))}
              </Form.Select>
              {selected && (
                <Form.Text>
                  {[selected.manufacturer, selected.serialNumber, deviceId].filter(Boolean).join(" / ")}
                </Form.Text>
              )}
            </Form.Group>

            <Form.Group className="mb-3">
              <Form.Label>ボーレート</Form.Label>
              <Form.Select value={baudRate} onChange={(event) => setBaudRate(Number(event.target.value))} disabled={busy}>
                {[9600, 19200, 38400, 57600, 115200].map((rate) => <option key={rate} value={rate}>{rate.toLocaleString()} bps</option>)}
              </Form.Select>
              <Form.Text>USB CDC機器では通常9600のままで利用できます。</Form.Text>
            </Form.Group>

            <div className="d-flex gap-2">
              {busy ? (
                <Button variant="outline-danger" onClick={disconnect}>切断</Button>
              ) : (
                <Button onClick={connect} disabled={!selectedPort || loading}>
                  <FontAwesomeIcon icon={faPlug} className="me-2" />接続して待受
                </Button>
              )}
            </div>

            {(actionError || status.error) && <div className="alert alert-danger small mt-3 mb-0">{actionError || status.error}</div>}
            {status.state === "connected" && (
              <div className="scanner-live mt-3">
                <span className="scanner-live-dot" />
                <div><strong>スキャン待機中</strong><br /><small>{status.portName} · {status.baudRate?.toLocaleString()} bps</small></div>
              </div>
            )}
          </Card.Body>
        </Card>
      </Col>

      <Col lg={7}>
        <Card className="h-100">
          <Card.Body>
            <div className="d-flex justify-content-between align-items-center mb-3">
              <div>
                <Card.Title className="mb-1">読み取り履歴</Card.Title>
                <Card.Text className="small text-muted mb-0">直近100件をこの画面内だけに保持します。</Card.Text>
              </div>
              <Button size="sm" variant="outline-secondary" onClick={() => setScans([])} disabled={scans.length === 0}>
                <FontAwesomeIcon icon={faTrash} className="me-1" />消去
              </Button>
            </div>
            {scans.length === 0 ? (
              <div className="scanner-empty">
                <FontAwesomeIcon icon={faQrcode} />
                <span>QRコードを読み取ると、ここに内容が表示されます</span>
              </div>
            ) : (
              <div className="table-responsive scanner-history">
                <Table hover size="sm" className="align-middle mb-0">
                  <thead><tr><th>時刻</th><th>内容</th><th>ポート</th></tr></thead>
                  <tbody>
                    {scans.map((scan, index) => (
                      <tr key={`${scan.scannedAt}-${index}`}>
                        <td className="text-nowrap small">{new Date(scan.scannedAt).toLocaleTimeString()}</td>
                        <td><code className="scanner-value">{scan.value}</code></td>
                        <td className="text-nowrap small text-muted">{scan.portName}</td>
                      </tr>
                    ))}
                  </tbody>
                </Table>
              </div>
            )}
          </Card.Body>
        </Card>
      </Col>
    </Row>
  );
}
