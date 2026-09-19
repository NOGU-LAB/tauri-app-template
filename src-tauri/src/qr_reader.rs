use serde::Serialize;
use std::{
    io::{ErrorKind, Read},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, Manager, State};

const READ_TIMEOUT: Duration = Duration::from_millis(150);
const MAX_SCAN_BYTES: usize = 4096;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QrReaderInfo {
    port_name: String,
    product: Option<String>,
    manufacturer: Option<String>,
    serial_number: Option<String>,
    vendor_id: Option<u16>,
    product_id: Option<u16>,
    is_likely_scanner: bool,
}

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QrReaderStatus {
    state: String,
    port_name: Option<String>,
    baud_rate: Option<u32>,
    error: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct QrScanEvent {
    value: String,
    port_name: String,
    scanned_at: u128,
}

struct ReaderSession {
    id: u64,
    cancel: Arc<AtomicBool>,
}

pub struct QrReaderState {
    session: Mutex<Option<ReaderSession>>,
    status: Mutex<QrReaderStatus>,
    next_session_id: AtomicU64,
}

impl Default for QrReaderState {
    fn default() -> Self {
        Self {
            session: Mutex::new(None),
            status: Mutex::new(QrReaderStatus {
                state: "disconnected".into(),
                ..QrReaderStatus::default()
            }),
            next_session_id: AtomicU64::new(0),
        }
    }
}

#[tauri::command]
pub fn list_qr_readers() -> Result<Vec<QrReaderInfo>, String> {
    let mut readers = serialport::available_ports()
        .map_err(|error| format!("シリアルポートを取得できません: {error}"))?
        .into_iter()
        .map(|port| {
            let (product, manufacturer, serial_number, vendor_id, product_id) = match port.port_type
            {
                serialport::SerialPortType::UsbPort(usb) => (
                    usb.product,
                    usb.manufacturer,
                    usb.serial_number,
                    Some(usb.vid),
                    Some(usb.pid),
                ),
                _ => (None, None, None, None, None),
            };
            let searchable = format!(
                "{} {} {} {}",
                port.port_name,
                product.as_deref().unwrap_or_default(),
                manufacturer.as_deref().unwrap_or_default(),
                serial_number.as_deref().unwrap_or_default()
            )
            .to_ascii_lowercase();
            let is_likely_scanner = ["scanner", "barcode", "qr", "sm-2d", "sm_2d"]
                .iter()
                .any(|marker| searchable.contains(marker));
            QrReaderInfo {
                port_name: port.port_name,
                product,
                manufacturer,
                serial_number,
                vendor_id,
                product_id,
                is_likely_scanner,
            }
        })
        .collect::<Vec<_>>();
    readers.sort_by_key(|reader| (!reader.is_likely_scanner, reader.port_name.clone()));
    Ok(readers)
}

#[tauri::command]
pub fn get_qr_reader_status(state: State<QrReaderState>) -> Result<QrReaderStatus, String> {
    state
        .status
        .lock()
        .map(|status| status.clone())
        .map_err(|_| "QRリーダー状態を取得できません".into())
}

#[tauri::command]
pub fn start_qr_reader(
    app: AppHandle,
    state: State<QrReaderState>,
    port_name: String,
    baud_rate: u32,
) -> Result<QrReaderStatus, String> {
    if !(1_200..=921_600).contains(&baud_rate) {
        return Err("ボーレートが範囲外です".into());
    }
    let available = serialport::available_ports()
        .map_err(|error| format!("シリアルポートを取得できません: {error}"))?;
    if !available.iter().any(|port| port.port_name == port_name) {
        return Err("選択したシリアルポートが見つかりません".into());
    }

    stop_active_session(&state)?;
    let id = state.next_session_id.fetch_add(1, Ordering::SeqCst) + 1;
    let cancel = Arc::new(AtomicBool::new(false));
    *state
        .session
        .lock()
        .map_err(|_| "QRリーダー状態を更新できません".to_string())? = Some(ReaderSession {
        id,
        cancel: cancel.clone(),
    });
    let connecting = QrReaderStatus {
        state: "connecting".into(),
        port_name: Some(port_name.clone()),
        baud_rate: Some(baud_rate),
        error: None,
    };
    set_status(&state, connecting.clone())?;
    let _ = app.emit("qr-reader-status", connecting.clone());

    thread::spawn(move || run_reader(app, id, cancel, port_name, baud_rate));
    Ok(connecting)
}

#[tauri::command]
pub fn stop_qr_reader(
    app: AppHandle,
    state: State<QrReaderState>,
) -> Result<QrReaderStatus, String> {
    stop_active_session(&state)?;
    let disconnected = QrReaderStatus {
        state: "disconnected".into(),
        ..QrReaderStatus::default()
    };
    set_status(&state, disconnected.clone())?;
    let _ = app.emit("qr-reader-status", disconnected.clone());
    Ok(disconnected)
}

pub fn shutdown(state: &QrReaderState) {
    if let Ok(mut session) = state.session.lock() {
        if let Some(session) = session.take() {
            session.cancel.store(true, Ordering::SeqCst);
        }
    }
}

fn run_reader(app: AppHandle, id: u64, cancel: Arc<AtomicBool>, port_name: String, baud_rate: u32) {
    let mut port = match serialport::new(&port_name, baud_rate)
        .timeout(READ_TIMEOUT)
        .open()
    {
        Ok(port) => port,
        Err(error) => {
            finish_with_error(&app, id, format!("{}を開けません: {error}", port_name));
            return;
        }
    };

    update_if_active(
        &app,
        id,
        QrReaderStatus {
            state: "connected".into(),
            port_name: Some(port_name.clone()),
            baud_rate: Some(baud_rate),
            error: None,
        },
    );

    let mut decoder = ScanDecoder::default();
    let mut buffer = [0_u8; 256];
    while !cancel.load(Ordering::SeqCst) {
        match port.read(&mut buffer) {
            Ok(count) if count > 0 => {
                for value in decoder.push(&buffer[..count]) {
                    let scan = QrScanEvent {
                        value,
                        port_name: port_name.clone(),
                        scanned_at: SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis(),
                    };
                    let _ = app.emit("qr-code-scanned", scan);
                }
            }
            Ok(_) => {}
            Err(error) if error.kind() == ErrorKind::TimedOut => {}
            Err(error) => {
                finish_with_error(&app, id, format!("読み取り中に切断されました: {error}"));
                return;
            }
        }
    }
}

fn finish_with_error(app: &AppHandle, id: u64, error: String) {
    update_if_active(
        app,
        id,
        QrReaderStatus {
            state: "error".into(),
            error: Some(error),
            ..QrReaderStatus::default()
        },
    );
}

fn update_if_active(app: &AppHandle, id: u64, status: QrReaderStatus) {
    let Some(state) = app.try_state::<QrReaderState>() else {
        return;
    };
    let active = state
        .session
        .lock()
        .ok()
        .and_then(|session| session.as_ref().map(|session| session.id == id))
        .unwrap_or(false);
    if active && set_status(&state, status.clone()).is_ok() {
        let _ = app.emit("qr-reader-status", status);
    }
}

fn stop_active_session(state: &QrReaderState) -> Result<(), String> {
    if let Some(session) = state
        .session
        .lock()
        .map_err(|_| "QRリーダー状態を更新できません".to_string())?
        .take()
    {
        session.cancel.store(true, Ordering::SeqCst);
    }
    Ok(())
}

fn set_status(state: &QrReaderState, status: QrReaderStatus) -> Result<(), String> {
    *state
        .status
        .lock()
        .map_err(|_| "QRリーダー状態を更新できません".to_string())? = status;
    Ok(())
}

#[derive(Default)]
struct ScanDecoder {
    buffer: Vec<u8>,
}

impl ScanDecoder {
    fn push(&mut self, bytes: &[u8]) -> Vec<String> {
        let mut scans = Vec::new();
        for byte in bytes {
            if *byte == b'\r' || *byte == b'\n' {
                if !self.buffer.is_empty() {
                    let value = String::from_utf8_lossy(&self.buffer).trim().to_string();
                    self.buffer.clear();
                    if !value.is_empty() {
                        scans.push(value);
                    }
                }
            } else if self.buffer.len() < MAX_SCAN_BYTES {
                self.buffer.push(*byte);
            } else {
                self.buffer.clear();
            }
        }
        scans
    }
}

#[cfg(test)]
mod tests {
    use super::ScanDecoder;

    #[test]
    fn decodes_fragmented_crlf_scans() {
        let mut decoder = ScanDecoder::default();
        assert!(decoder.push(b"https://exam").is_empty());
        assert_eq!(
            decoder.push(b"ple.test\r\nABC-123\r"),
            ["https://example.test", "ABC-123"]
        );
    }

    #[test]
    fn ignores_empty_lines_and_trims_whitespace() {
        let mut decoder = ScanDecoder::default();
        assert_eq!(decoder.push(b"\r\n  value  \n"), ["value"]);
    }
}
