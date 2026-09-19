use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, Manager, State};

#[cfg(not(target_os = "windows"))]
use std::process::Command;

const MAX_PDF_SIZE: usize = 10 << 20;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrinterInfo {
    name: String,
    is_default: bool,
    is_offline: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrintJob {
    id: String,
    printer_name: String,
    document_name: String,
    status: String,
    message: String,
    cancelable: bool,
}

#[derive(Default)]
pub struct PrintState {
    jobs: Mutex<HashMap<String, PrintJob>>,
    cancel_requested: Mutex<HashSet<String>>,
}

#[tauri::command]
pub fn list_printers() -> Result<Vec<PrinterInfo>, String> {
    platform_list_printers()
}

#[tauri::command]
pub fn start_print_job(
    app: AppHandle,
    state: State<PrintState>,
    printer_name: String,
    document_name: String,
    pdf_base64: String,
) -> Result<PrintJob, String> {
    let printers = platform_list_printers()?;
    let printer = printers
        .iter()
        .find(|printer| printer.name == printer_name)
        .ok_or_else(|| "選択したプリンターが見つかりません".to_string())?;
    if printer.is_offline {
        return Err("選択したプリンターはオフラインです".into());
    }
    let pdf = STANDARD
        .decode(pdf_base64)
        .map_err(|_| "PDFデータが不正です".to_string())?;
    if pdf.len() > MAX_PDF_SIZE || !pdf.starts_with(b"%PDF-") {
        return Err("10MB以下のPDFを指定してください".into());
    }

    let id = unique_id();
    let path = temporary_pdf_path(&id);
    fs::create_dir_all(path.parent().expect("temporary PDF has parent"))
        .map_err(|error| format!("一時フォルダーを作成できません: {error}"))?;
    fs::write(&path, pdf).map_err(|error| format!("一時PDFを保存できません: {error}"))?;

    let safe_document_name = Path::new(&document_name)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("sample-report.pdf")
        .to_string();
    let job = PrintJob {
        id: id.clone(),
        printer_name: printer_name.clone(),
        document_name: safe_document_name,
        status: "queued".into(),
        message: "OSへ印刷ジョブを投入しています".into(),
        cancelable: true,
    };
    state
        .jobs
        .lock()
        .map_err(|_| "印刷状態を取得できません".to_string())?
        .insert(id.clone(), job.clone());

    thread::spawn(move || run_print_job(app, id, printer_name, path));
    Ok(job)
}

#[tauri::command]
pub fn get_print_job(state: State<PrintState>, id: String) -> Result<PrintJob, String> {
    state
        .jobs
        .lock()
        .map_err(|_| "印刷状態を取得できません".to_string())?
        .get(&id)
        .cloned()
        .ok_or_else(|| "印刷ジョブが見つかりません".into())
}

#[tauri::command]
pub fn cancel_print_job(
    app: AppHandle,
    state: State<PrintState>,
    id: String,
) -> Result<PrintJob, String> {
    let mut jobs = state
        .jobs
        .lock()
        .map_err(|_| "印刷状態を取得できません".to_string())?;
    let job = jobs
        .get_mut(&id)
        .ok_or_else(|| "印刷ジョブが見つかりません".to_string())?;
    if !job.cancelable {
        return Err("この印刷ジョブはキャンセルできる段階を過ぎています".into());
    }
    job.status = "cancelled".into();
    job.message = "キャンセルを要求しました（プリンター側で既に処理済みの場合を除きます）".into();
    job.cancelable = false;
    let updated = job.clone();
    drop(jobs);
    state
        .cancel_requested
        .lock()
        .map_err(|_| "キャンセル状態を更新できません".to_string())?
        .insert(id);
    let _ = app.emit("print-job-updated", &updated);
    Ok(updated)
}

fn run_print_job(app: AppHandle, id: String, printer_name: String, path: PathBuf) {
    update_job(
        &app,
        &id,
        "printing",
        "OSの印刷システムへ引き渡しました。PDFプリンターでは保存先を選んでください。",
        true,
    );
    let result = platform_print_and_wait(&app, &id, &printer_name, &path);
    if !is_cancelled(&app, &id) {
        match result {
            Ok(()) => update_job(
                &app,
                &id,
                "completed",
                "OSの印刷キューで処理が完了しました",
                false,
            ),
            Err(error) => update_job(&app, &id, "failed", &error, false),
        }
    }
    let _ = fs::remove_file(path);
}

fn update_job(app: &AppHandle, id: &str, status: &str, message: &str, cancelable: bool) {
    let Some(state) = app.try_state::<PrintState>() else {
        return;
    };
    let Ok(mut jobs) = state.jobs.lock() else {
        return;
    };
    let Some(job) = jobs.get_mut(id) else {
        return;
    };
    job.status = status.into();
    job.message = message.into();
    job.cancelable = cancelable;
    let updated = job.clone();
    drop(jobs);
    let _ = app.emit("print-job-updated", updated);
}

fn is_cancelled(app: &AppHandle, id: &str) -> bool {
    app.try_state::<PrintState>()
        .and_then(|state| {
            state
                .cancel_requested
                .lock()
                .ok()
                .map(|cancelled| cancelled.contains(id))
        })
        .unwrap_or(false)
}

fn unique_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("print-{nanos}-{}", std::process::id())
}

fn temporary_pdf_path(id: &str) -> PathBuf {
    std::env::temp_dir()
        .join("tauri-app-template")
        .join(format!("{id}.pdf"))
}

#[cfg(target_os = "windows")]
fn platform_list_printers() -> Result<Vec<PrinterInfo>, String> {
    use std::ptr;
    use windows_sys::Win32::Graphics::Printing::{
        EnumPrintersW, GetDefaultPrinterW, PRINTER_ATTRIBUTE_WORK_OFFLINE,
        PRINTER_ENUM_CONNECTIONS, PRINTER_ENUM_LOCAL, PRINTER_INFO_4W,
    };

    let default_name = unsafe {
        let mut length = 0;
        GetDefaultPrinterW(ptr::null_mut(), &mut length);
        if length == 0 {
            String::new()
        } else {
            let mut buffer = vec![0u16; length as usize];
            if GetDefaultPrinterW(buffer.as_mut_ptr(), &mut length) == 0 {
                String::new()
            } else {
                String::from_utf16_lossy(&buffer[..length.saturating_sub(1) as usize])
            }
        }
    };

    let mut needed = 0;
    let mut returned = 0;
    let flags = PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS;
    unsafe {
        EnumPrintersW(
            flags,
            ptr::null(),
            4,
            ptr::null_mut(),
            0,
            &mut needed,
            &mut returned,
        );
    }
    if needed == 0 {
        return Ok(Vec::new());
    }
    let word_size = std::mem::size_of::<usize>();
    let mut buffer = vec![0usize; (needed as usize).div_ceil(word_size)];
    let success = unsafe {
        EnumPrintersW(
            flags,
            ptr::null(),
            4,
            buffer.as_mut_ptr().cast(),
            needed,
            &mut needed,
            &mut returned,
        )
    };
    if success == 0 {
        return Err(format!(
            "Windows印刷APIがプリンター一覧を返しませんでした: {}",
            std::io::Error::last_os_error()
        ));
    }
    let entries = unsafe {
        std::slice::from_raw_parts(buffer.as_ptr().cast::<PRINTER_INFO_4W>(), returned as usize)
    };
    let mut printers = entries
        .iter()
        .filter_map(|entry| {
            let name = unsafe { wide_ptr_string(entry.pPrinterName) };
            if name.is_empty() {
                return None;
            }
            Some(PrinterInfo {
                is_default: name.eq_ignore_ascii_case(&default_name),
                is_offline: entry.Attributes & PRINTER_ATTRIBUTE_WORK_OFFLINE != 0,
                name,
            })
        })
        .collect::<Vec<_>>();
    printers.sort_by_key(|printer| printer.name.to_lowercase());
    Ok(printers)
}

#[cfg(target_os = "windows")]
fn platform_print_and_wait(
    app: &AppHandle,
    id: &str,
    printer_name: &str,
    path: &Path,
) -> Result<(), String> {
    let before = windows_queue_ids(printer_name)?;
    windows_shell_print_to(path, printer_name)?;
    let mut observed = HashSet::new();
    for _ in 0..240 {
        if is_cancelled(app, id) {
            for job_id in &observed {
                let _ = windows_remove_job(printer_name, *job_id);
            }
            return Ok(());
        }
        thread::sleep(Duration::from_millis(500));
        let current = windows_queue_ids(printer_name)?;
        observed.extend(current.difference(&before));
        if !observed.is_empty() && observed.is_disjoint(&current) {
            return Ok(());
        }
    }
    Err("印刷キューを120秒以内に確認できませんでした。PDF保存ダイアログを閉じた場合は再実行してください。".into())
}

#[cfg(target_os = "windows")]
fn windows_queue_ids(printer_name: &str) -> Result<HashSet<u32>, String> {
    use std::ptr;
    use windows_sys::Win32::Graphics::Printing::{EnumJobsW, JOB_INFO_1W};

    let handle = WindowsPrinterHandle::open(printer_name)?;
    let mut needed = 0;
    let mut returned = 0;
    unsafe {
        EnumJobsW(
            handle.0,
            0,
            u32::MAX,
            1,
            ptr::null_mut(),
            0,
            &mut needed,
            &mut returned,
        );
    }
    if needed == 0 {
        return Ok(HashSet::new());
    }
    let word_size = std::mem::size_of::<usize>();
    let mut buffer = vec![0usize; (needed as usize).div_ceil(word_size)];
    let success = unsafe {
        EnumJobsW(
            handle.0,
            0,
            u32::MAX,
            1,
            buffer.as_mut_ptr().cast(),
            needed,
            &mut needed,
            &mut returned,
        )
    };
    if success == 0 {
        return Err(format!(
            "Windows印刷キューを取得できません: {}",
            std::io::Error::last_os_error()
        ));
    }
    let entries = unsafe {
        std::slice::from_raw_parts(buffer.as_ptr().cast::<JOB_INFO_1W>(), returned as usize)
    };
    Ok(entries.iter().map(|entry| entry.JobId).collect())
}

#[cfg(target_os = "windows")]
fn windows_remove_job(printer_name: &str, job_id: u32) -> Result<(), String> {
    use std::ptr;
    use windows_sys::Win32::Graphics::Printing::{SetJobW, JOB_CONTROL_CANCEL};

    let handle = WindowsPrinterHandle::open(printer_name)?;
    let success = unsafe { SetJobW(handle.0, job_id, 0, ptr::null(), JOB_CONTROL_CANCEL) };
    if success == 0 {
        return Err(format!(
            "Windows印刷ジョブをキャンセルできません: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
struct WindowsPrinterHandle(windows_sys::Win32::Foundation::HANDLE);

#[cfg(target_os = "windows")]
impl WindowsPrinterHandle {
    fn open(printer_name: &str) -> Result<Self, String> {
        use std::{ffi::OsStr, os::windows::ffi::OsStrExt, ptr};
        use windows_sys::Win32::Graphics::Printing::OpenPrinterW;

        let name = OsStr::new(printer_name)
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>();
        let mut handle = ptr::null_mut();
        if unsafe { OpenPrinterW(name.as_ptr(), &mut handle, ptr::null()) } == 0 {
            return Err(format!(
                "Windowsプリンターを開けません: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(Self(handle))
    }
}

#[cfg(target_os = "windows")]
impl Drop for WindowsPrinterHandle {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Graphics::Printing::ClosePrinter(self.0);
        }
    }
}

#[cfg(target_os = "windows")]
unsafe fn wide_ptr_string(value: *const u16) -> String {
    if value.is_null() {
        return String::new();
    }
    let mut length = 0;
    while unsafe { *value.add(length) } != 0 {
        length += 1;
    }
    String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(value, length) })
}

#[cfg(target_os = "windows")]
fn windows_shell_print_to(path: &Path, printer_name: &str) -> Result<(), String> {
    use std::{ffi::OsStr, os::windows::ffi::OsStrExt, ptr};
    use windows_sys::Win32::UI::Shell::ShellExecuteW;

    fn wide(value: &OsStr) -> Vec<u16> {
        value.encode_wide().chain(Some(0)).collect()
    }
    let operation = wide(OsStr::new("printto"));
    let file = wide(path.as_os_str());
    let parameters = wide(OsStr::new(&format!("\"{printer_name}\"")));
    let result = unsafe {
        ShellExecuteW(
            ptr::null_mut(),
            operation.as_ptr(),
            file.as_ptr(),
            parameters.as_ptr(),
            ptr::null(),
            1,
        )
    } as isize;
    if result <= 32 {
        return Err(format!(
            "PDFのprintto操作を開始できませんでした (ShellExecute={result})"
        ));
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn platform_list_printers() -> Result<Vec<PrinterInfo>, String> {
    let output = Command::new("lpstat")
        .args(["-p", "-d"])
        .output()
        .map_err(|error| format!("lpstatを起動できません: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let default_name = stdout
        .lines()
        .find_map(|line| line.strip_prefix("system default destination: "))
        .unwrap_or_default();
    Ok(stdout
        .lines()
        .filter_map(|line| {
            let name = line.strip_prefix("printer ")?.split_whitespace().next()?;
            Some(PrinterInfo {
                name: name.into(),
                is_default: name == default_name,
                is_offline: line.contains("disabled"),
            })
        })
        .collect())
}

#[cfg(not(target_os = "windows"))]
fn platform_print_and_wait(
    app: &AppHandle,
    id: &str,
    printer_name: &str,
    path: &Path,
) -> Result<(), String> {
    let output = Command::new("lp")
        .args(["-d", printer_name])
        .arg(path)
        .output()
        .map_err(|error| format!("lpを起動できません: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "印刷ジョブの投入に失敗しました: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let response = String::from_utf8_lossy(&output.stdout);
    let request_id = response
        .split_whitespace()
        .find(|part| {
            part.rsplit_once('-')
                .is_some_and(|(_, value)| value.parse::<u32>().is_ok())
        })
        .ok_or_else(|| format!("印刷ジョブIDを取得できません: {}", response.trim()))?
        .to_string();
    for _ in 0..240 {
        if is_cancelled(app, id) {
            let _ = Command::new("cancel").arg(&request_id).output();
            return Ok(());
        }
        thread::sleep(Duration::from_millis(500));
        let status = Command::new("lpstat")
            .args(["-W", "not-completed", "-o", &request_id])
            .output()
            .map_err(|error| format!("印刷状態を取得できません: {error}"))?;
        if status.stdout.is_empty() {
            return Ok(());
        }
    }
    Err("印刷キューが120秒以内に完了しませんでした".into())
}
