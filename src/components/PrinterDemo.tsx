import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Alert, Badge, Button, Card, Form, Modal, Spinner, Stack } from "react-bootstrap";
import { requestJSON } from "../api";

type Props = { apiBase: string; token: string };
type Printer = { name: string; isDefault: boolean; isOffline: boolean };
type Report = { fileName: string; title: string; pdfBase64: string; size: number };
type PrintStatus = "queued" | "printing" | "completed" | "cancelled" | "failed";
type PrintJob = {
  id: string;
  printerName: string;
  documentName: string;
  status: PrintStatus;
  message: string;
  cancelable: boolean;
};

const statusLabel: Record<PrintStatus, string> = {
  queued: "投入中",
  printing: "印刷中",
  completed: "完了",
  cancelled: "キャンセル",
  failed: "失敗",
};

const statusVariant: Record<PrintStatus, string> = {
  queued: "secondary",
  printing: "primary",
  completed: "success",
  cancelled: "warning",
  failed: "danger",
};

function errorMessage(cause: unknown, fallback: string) {
  return cause instanceof Error ? cause.message : String(cause || fallback);
}

function pdfBlobUrl(base64: string) {
  const binary = window.atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) bytes[index] = binary.charCodeAt(index);
  return URL.createObjectURL(new Blob([bytes], { type: "application/pdf" }));
}

export function PrinterDemo({ apiBase, token }: Props) {
  const [printers, setPrinters] = useState<Printer[]>([]);
  const [selectedPrinter, setSelectedPrinter] = useState("");
  const [report, setReport] = useState<Report | null>(null);
  const [job, setJob] = useState<PrintJob | null>(null);
  const [previewOpen, setPreviewOpen] = useState(false);
  const [loadingPrinters, setLoadingPrinters] = useState(true);
  const [generating, setGenerating] = useState(false);
  const [error, setError] = useState("");
  const previewUrl = useMemo(() => report ? pdfBlobUrl(report.pdfBase64) : "", [report]);

  useEffect(() => () => { if (previewUrl) URL.revokeObjectURL(previewUrl); }, [previewUrl]);

  async function refreshPrinters() {
    setLoadingPrinters(true);
    try {
      const values = await invoke<Printer[]>("list_printers");
      setPrinters(values);
      setSelectedPrinter((current) => {
        if (values.some((printer) => printer.name === current)) return current;
        return values.find((printer) => printer.isDefault && !printer.isOffline)?.name
          ?? values.find((printer) => !printer.isOffline)?.name
          ?? values.find((printer) => printer.isDefault)?.name
          ?? values[0]?.name
          ?? "";
      });
      setError("");
    } catch (cause) {
      setError(errorMessage(cause, "プリンター一覧を取得できませんでした"));
    } finally {
      setLoadingPrinters(false);
    }
  }

  useEffect(() => { void refreshPrinters(); }, []);

  useEffect(() => {
    const stop = listen<PrintJob>("print-job-updated", ({ payload }) => {
      setJob((current) => current?.id === payload.id ? payload : current);
    });
    return () => { void stop.then((unlisten) => unlisten()); };
  }, []);

  useEffect(() => {
    if (!job || (job.status !== "queued" && job.status !== "printing")) return;
    let disposed = false;
    const timer = window.setInterval(async () => {
      try {
        const latest = await invoke<PrintJob>("get_print_job", { id: job.id });
        if (!disposed) setJob(latest);
      } catch (cause) {
        if (!disposed) setError(errorMessage(cause, "印刷状態を取得できませんでした"));
      }
    }, 500);
    return () => {
      disposed = true;
      window.clearInterval(timer);
    };
  }, [job?.id, job?.status]);

  async function generateReport() {
    setGenerating(true);
    try {
      const generated = await requestJSON<Report>(apiBase, token, "/api/reports/sample");
      setReport(generated);
      setJob(null);
      setError("");
      return generated;
    } catch (cause) {
      setError(errorMessage(cause, "帳票を生成できませんでした"));
      return null;
    } finally {
      setGenerating(false);
    }
  }

  async function showPreview() {
    const generated = report ?? await generateReport();
    if (generated) setPreviewOpen(true);
  }

  async function printReport() {
    if (!selectedPrinter) return;
    const generated = report ?? await generateReport();
    if (!generated) return;
    try {
      setJob(await invoke<PrintJob>("start_print_job", {
        printerName: selectedPrinter,
        documentName: generated.fileName,
        pdfBase64: generated.pdfBase64,
      }));
      setError("");
    } catch (cause) {
      setError(errorMessage(cause, "印刷ジョブを開始できませんでした"));
    }
  }

  async function cancelPrint() {
    if (!job) return;
    try {
      setJob(await invoke<PrintJob>("cancel_print_job", { id: job.id }));
    } catch (cause) {
      setError(errorMessage(cause, "印刷ジョブをキャンセルできませんでした"));
    }
  }

  const selected = printers.find((printer) => printer.name === selectedPrinter);
  const defaultPrinter = printers.find((printer) => printer.isDefault);
  const active = job?.status === "queued" || job?.status === "printing";

  return (
    <>
      {error && <Alert variant="danger" dismissible onClose={() => setError("")}>{error}</Alert>}
      <Card className="shadow-sm mb-4">
        <Card.Body>
          <div className="d-flex flex-wrap justify-content-between gap-3 align-items-start mb-3">
            <div>
              <Card.Title className="h5 mb-1">PDF帳票とOSプリンター</Card.Title>
              <Card.Text className="text-muted small mb-0">PDF生成はGo、OSの列挙と印刷キュー操作はRustが担当します。</Card.Text>
            </div>
            <Badge bg="primary" className="px-3 py-2">Go → Rust → OS</Badge>
          </div>

          <Form.Group className="mb-3">
            <div className="d-flex justify-content-between align-items-center mb-2">
              <Form.Label className="mb-0">プリンター</Form.Label>
              <Button size="sm" variant="outline-secondary" onClick={refreshPrinters} disabled={loadingPrinters || active}>再読込</Button>
            </div>
            {loadingPrinters ? <div className="text-muted small"><Spinner size="sm" className="me-2" />取得中…</div> : (
              <Form.Select value={selectedPrinter} onChange={(event) => setSelectedPrinter(event.target.value)} disabled={active || printers.length === 0}>
                {printers.length === 0 && <option value="">利用可能なプリンターがありません</option>}
                {printers.map((printer) => (
                  <option key={printer.name} value={printer.name} disabled={printer.isOffline}>
                    {printer.name}{printer.isDefault ? "（既定）" : ""}{printer.isOffline ? "（オフライン）" : ""}
                  </option>
                ))}
              </Form.Select>
            )}
            {selected && <Form.Text>
              {selected.isDefault
                ? "OSの既定プリンターです。"
                : defaultPrinter
                  ? `OSの既定: ${defaultPrinter.name}${defaultPrinter.isOffline ? "（オフライン）" : ""}`
                  : "OSの既定プリンターは設定されていません。"}
            </Form.Text>}
          </Form.Group>

          <Stack direction="horizontal" gap={2} className="flex-wrap">
            <Button variant="outline-primary" onClick={generateReport} disabled={generating || active}>{generating ? "生成中…" : "GoでPDF生成"}</Button>
            <Button variant="outline-dark" onClick={showPreview} disabled={generating || active}>プレビューのみ</Button>
            <Button onClick={printReport} disabled={!selectedPrinter || selected?.isOffline || generating || active}>選択プリンターで印刷</Button>
            {job?.cancelable && <Button variant="outline-danger" onClick={cancelPrint}>キャンセル</Button>}
          </Stack>

          {report && <Alert variant="light" className="border mt-3 mb-0 small">{report.fileName}（{Math.ceil(report.size / 1024)} KB）をGoで生成済み</Alert>}
          {job && (
            <div className="print-status mt-3" aria-live="polite">
              <div className="d-flex justify-content-between align-items-center gap-3">
                <strong>{job.printerName}</strong>
                <Badge bg={statusVariant[job.status]}>{statusLabel[job.status]}</Badge>
              </div>
              <div className="small text-muted mt-2">{job.message}</div>
            </div>
          )}
        </Card.Body>
      </Card>

      <Alert variant="info" className="small">
        <strong>キャンセル範囲:</strong> OSキューに残っている間は取り消しを試みます。プリンタードライバーが受信済み、物理印刷開始後、または「Microsoft Print to PDF」で保存完了後は取り消せません。プレビューのみでは印刷ジョブを投入しません。
      </Alert>

      <Modal show={previewOpen} onHide={() => setPreviewOpen(false)} size="xl" centered>
        <Modal.Header closeButton><Modal.Title className="h5">印刷しないPDFプレビュー</Modal.Title></Modal.Header>
        <Modal.Body className="p-0">
          {previewUrl && <iframe className="pdf-preview" title={report?.title ?? "PDF preview"} src={previewUrl} />}
        </Modal.Body>
        <Modal.Footer><Button variant="secondary" onClick={() => setPreviewOpen(false)}>閉じる</Button></Modal.Footer>
      </Modal>
    </>
  );
}
