import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { Alert, Badge, Button, Card, ProgressBar, Stack } from "react-bootstrap";
import { requestJSON } from "../api";

type ImportedFile = { name: string; path: string; content: string; size: number };
type JobStatus = "queued" | "running" | "completed" | "cancelled" | "failed";
type Job = {
  id: string;
  fileName: string;
  status: JobStatus;
  progress: number;
  processed: number;
  total: number;
  result?: string;
  error?: string;
};
type Props = { apiBase: string; token: string };

const activeStatuses = new Set<JobStatus>(["queued", "running"]);
const errorMessage = (cause: unknown, fallback: string) => cause instanceof Error ? cause.message : String(cause || fallback);

export function DesktopDemo({ apiBase, token }: Props) {
  const [file, setFile] = useState<ImportedFile | null>(null);
  const [job, setJob] = useState<Job | null>(null);
  const [error, setError] = useState("");
  const [savedPath, setSavedPath] = useState("");
  const [isPicking, setIsPicking] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [isDragging, setIsDragging] = useState(false);
  const notifiedJobs = useRef(new Set<string>());

  function applyFile(selected: ImportedFile) {
    setFile(selected);
    setJob(null);
    setSavedPath("");
    setError("");
  }

  async function loadDroppedFile(path: string) {
    try {
      applyFile(await invoke<ImportedFile>("read_dropped_file", { path }));
    } catch (cause) {
      setError(errorMessage(cause, "ドロップしたファイルを読み込めませんでした"));
    }
  }

  useEffect(() => {
    let disposed = false;
    const unlisten = getCurrentWebview().onDragDropEvent((event) => {
      if (disposed) return;
      if (event.payload.type === "enter" || event.payload.type === "over") setIsDragging(true);
      if (event.payload.type === "leave") setIsDragging(false);
      if (event.payload.type === "drop") {
        setIsDragging(false);
        const [path] = event.payload.paths;
        if (path) void loadDroppedFile(path);
      }
    });
    return () => {
      disposed = true;
      unlisten.then((stop) => stop());
    };
  }, []);

  useEffect(() => {
    if (!job || !activeStatuses.has(job.status)) return;
    let disposed = false;
    let polling = false;
    const timer = window.setInterval(async () => {
      if (polling) return;
      polling = true;
      try {
        const latest = await requestJSON<Job>(apiBase, token, `/api/jobs/${job.id}`);
        if (disposed) return;
        setJob(latest);
        if (latest.status === "completed" && !notifiedJobs.current.has(latest.id)) {
          notifiedJobs.current.add(latest.id);
          await invoke("notify_if_hidden", {
            title: "処理が完了しました",
            body: `${latest.fileName} の変換結果を保存できます。`,
          });
        }
      } catch (cause) {
        if (!disposed) setError(errorMessage(cause, "進捗の取得に失敗しました"));
      } finally {
        polling = false;
      }
    }, 200);
    return () => {
      disposed = true;
      window.clearInterval(timer);
    };
  }, [apiBase, token, job?.id, job?.status]);

  async function pickFile() {
    setIsPicking(true);
    try {
      const selected = await invoke<ImportedFile | null>("pick_import_file");
      if (selected) applyFile(selected);
    } catch (cause) {
      setError(errorMessage(cause, "ファイル選択に失敗しました"));
    } finally {
      setIsPicking(false);
    }
  }

  async function startJob() {
    if (!file) return;
    setError("");
    setSavedPath("");
    try {
      setJob(await requestJSON<Job>(apiBase, token, "/api/jobs", {
        method: "POST",
        body: JSON.stringify({ fileName: file.name, content: file.content }),
      }));
    } catch (cause) {
      setError(errorMessage(cause, "バックグラウンド処理を開始できませんでした"));
    }
  }

  async function cancelJob() {
    if (!job) return;
    try {
      setJob(await requestJSON<Job>(apiBase, token, `/api/jobs/${job.id}`, { method: "DELETE" }));
    } catch (cause) {
      setError(errorMessage(cause, "キャンセルに失敗しました"));
    }
  }

  async function saveResult() {
    if (!job?.result) return;
    setIsSaving(true);
    try {
      const stem = file?.name.replace(/\.[^.]+$/, "") || "processed";
      const path = await invoke<string | null>("save_export_file", {
        suggestedName: `${stem}-processed.json`,
        content: job.result,
      });
      if (path) {
        setSavedPath(path);
        setError("");
      }
    } catch (cause) {
      setError(errorMessage(cause, "保存に失敗しました"));
    } finally {
      setIsSaving(false);
    }
  }

  const running = Boolean(job && activeStatuses.has(job.status));

  return (
    <div>
      {error && <Alert variant="danger" dismissible onClose={() => setError("")}>{error}</Alert>}
      <Card className="shadow-sm mb-4">
        <Card.Body>
          <div className="d-flex flex-wrap justify-content-between gap-3 align-items-start mb-3">
            <div>
              <Card.Title className="h5 mb-1">CSV / JSON バックグラウンド処理</Card.Title>
              <Card.Text className="text-muted small mb-0">OS連携はRust、データ処理とキャンセルはGoが担当します。</Card.Text>
            </div>
            <Badge bg="primary" className="px-3 py-2">Rust → Go</Badge>
          </div>
          <button type="button" className={`drop-zone w-100 ${isDragging ? "is-dragging" : ""}`} onClick={pickFile} disabled={running || isPicking}>
            <span className="drop-zone-icon" aria-hidden="true">⇩</span>
            <strong>{isPicking ? "ダイアログを開いています…" : "クリックして選択、またはここへドロップ"}</strong>
            <span>UTF-8のCSV / JSON、最大5MB</span>
          </button>
          {file && (
            <div className="selected-file mt-3">
              <div className="min-w-0">
                <strong className="d-block text-truncate">{file.name}</strong>
                <span className="text-muted small text-break">{file.path}</span>
              </div>
              <Badge bg="light" text="dark">{Math.max(1, Math.ceil(file.size / 1024))} KB</Badge>
            </div>
          )}
          <Stack direction="horizontal" gap={2} className="mt-3">
            <Button onClick={startJob} disabled={!file || running}>{running ? "処理中…" : "Goで処理を開始"}</Button>
            {running && <Button variant="outline-danger" onClick={cancelJob}>キャンセル</Button>}
            {job?.status === "completed" && <Button variant="success" onClick={saveResult} disabled={isSaving}>{isSaving ? "保存中…" : "結果を保存"}</Button>}
          </Stack>
          {job && (
            <div className="mt-4">
              <div className="d-flex justify-content-between small mb-2">
                <span>状態: <strong>{job.status}</strong></span><span>{job.processed} / {job.total}</span>
              </div>
              <ProgressBar now={job.progress} label={`${job.progress}%`} variant={job.status === "cancelled" ? "warning" : job.status === "failed" ? "danger" : "primary"} animated={running} />
              {job.error && <p className="text-danger small mt-2 mb-0">{job.error}</p>}
            </div>
          )}
          {savedPath && <Alert variant="success" className="mt-3 mb-0 small">保存しました: {savedPath}</Alert>}
        </Card.Body>
      </Card>
      <div className="row g-3">
        <div className="col-md-6"><Card className="h-100 border-0 bg-body-tertiary"><Card.Body>
          <Card.Title className="h6">通知テスト</Card.Title>
          <Card.Text className="small text-muted mb-0">処理中にウィンドウを最小化または非表示にすると、完了時にRustからOS通知を送ります。</Card.Text>
        </Card.Body></Card></div>
        <div className="col-md-6"><Card className="h-100 border-0 bg-body-tertiary"><Card.Body>
          <Card.Title className="h6">プリンター連携</Card.Title>
          <Card.Text className="small text-muted mb-0">PrinterタブでGoのPDF生成、Rustのプリンター列挙・印刷キュー操作、印刷しないプレビューを試せます。</Card.Text>
        </Card.Body></Card></div>
      </div>
    </div>
  );
}
