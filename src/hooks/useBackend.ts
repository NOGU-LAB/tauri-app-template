import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

type BackendInfo = {
  port: number;
  token: string;
};

export function useBackend() {
  const [apiBase, setApiBase] = useState<string>("");
  const [token, setToken] = useState<string>("");
  const [isReady, setIsReady] = useState(false);
  const [backendError, setBackendError] = useState("");

  useEffect(() => {
    let cancelled = false;

    function applyInfo(info: BackendInfo) {
      if (!cancelled) {
        setApiBase(`http://127.0.0.1:${info.port}`);
        setToken(info.token);
        setIsReady(true);
        setBackendError("");
      }
    }

    // イベント監視（Goが起動後に発火）
    const unlistenReady = listen<BackendInfo>("backend-ready", (event) => {
      applyInfo(event.payload);
    });
    const unlistenError = listen<string>("backend-error", (event) => {
      if (!cancelled) {
        setIsReady(false);
        setBackendError(event.payload);
      }
    });

    // イベントを見逃した場合のフォールバック：コマンドでポーリング
    let attempts = 0;
    const poll = setInterval(async () => {
      attempts += 1;
      try {
        const info = await invoke<BackendInfo | null>("get_backend_info");
        if (info) {
          applyInfo(info);
          clearInterval(poll);
        }
      } catch {
        // Tauriコマンドが使えない環境（ブラウザ等）は無視
      }
      if (attempts >= 50) {
        clearInterval(poll);
        if (!cancelled) {
          setBackendError("バックエンドの起動がタイムアウトしました");
        }
      }
    }, 300);

    return () => {
      cancelled = true;
      clearInterval(poll);
      unlistenReady.then((fn) => fn());
      unlistenError.then((fn) => fn());
    };
  }, []);

  return { apiBase, token, isReady, backendError };
}
