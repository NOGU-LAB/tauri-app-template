import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

export function useBackend() {
  const [apiBase, setApiBase] = useState<string>("");
  const [isReady, setIsReady] = useState(false);

  useEffect(() => {
    let cancelled = false;

    function applyPort(port: number) {
      if (!cancelled) {
        setApiBase(`http://localhost:${port}`);
        setIsReady(true);
      }
    }

    // イベント監視（Goが起動後に発火）
    const unlistenPromise = listen<number>("backend-ready", (event) => {
      applyPort(event.payload);
    });

    // イベントを見逃した場合のフォールバック：コマンドでポーリング
    const poll = setInterval(async () => {
      try {
        const port = await invoke<number | null>("get_backend_port");
        if (port) {
          applyPort(port);
          clearInterval(poll);
        }
      } catch {
        // Tauriコマンドが使えない環境（ブラウザ等）は無視
      }
    }, 300);

    return () => {
      cancelled = true;
      clearInterval(poll);
      unlistenPromise.then((fn) => fn());
    };
  }, []);

  return { apiBase, isReady };
}
