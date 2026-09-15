//! Cross-platform publishing from the same standalone HTML runtime used by
//! UniPPT's lossless player. No Microsoft Office installation is involved.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static EXPORT_PROGRESS: OnceLock<Mutex<HashMap<String, TrackedExportProgress>>> = OnceLock::new();
static RENDER_DAEMON: OnceLock<Mutex<Option<RendererDaemon>>> = OnceLock::new();
const PROGRESS_RETENTION: Duration = Duration::from_secs(30 * 60);

struct RendererDaemon {
    child: Child,
    endpoint: String,
}

enum DaemonRunError {
    Unavailable(String),
    Render(String),
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExportProgress {
    pub(crate) job_id: String,
    pub(crate) stage: String,
    pub(crate) percent: u8,
    pub(crate) message: String,
    pub(crate) completed_slides: usize,
    pub(crate) total_slides: usize,
    pub(crate) cached_slides: usize,
    pub(crate) frames: u64,
    pub(crate) done: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) error: Option<String>,
    pub(crate) elapsed_ms: u64,
}

struct TrackedExportProgress {
    progress: ExportProgress,
    started: Instant,
    updated: Instant,
}

fn progress_store() -> &'static Mutex<HashMap<String, TrackedExportProgress>> {
    EXPORT_PROGRESS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn with_progress_store<T>(
    callback: impl FnOnce(&mut HashMap<String, TrackedExportProgress>) -> T,
) -> T {
    let mut store = progress_store()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    store.retain(|_, entry| entry.updated.elapsed() <= PROGRESS_RETENTION);
    callback(&mut store)
}

pub(crate) fn valid_job_id(value: &str) -> bool {
    let length = value.len();
    (8..=96).contains(&length)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

pub(crate) fn begin_progress(job_id: &str, format: ExportFormat) {
    if !valid_job_id(job_id) {
        return;
    }
    let now = Instant::now();
    let message = match format {
        ExportFormat::VideoFast => "正在准备极速视频导出…",
        ExportFormat::Video => "正在准备高保真视频导出…",
        ExportFormat::Pdf => "正在准备 PDF 导出…",
        ExportFormat::ImagesZip => "正在准备幻灯片图片导出…",
    };
    with_progress_store(|store| {
        store.insert(
            job_id.to_string(),
            TrackedExportProgress {
                progress: ExportProgress {
                    job_id: job_id.to_string(),
                    stage: "preparing".into(),
                    percent: 2,
                    message: message.into(),
                    completed_slides: 0,
                    total_slides: 0,
                    cached_slides: 0,
                    frames: 0,
                    done: false,
                    error: None,
                    elapsed_ms: 0,
                },
                started: now,
                updated: now,
            },
        );
    });
}

pub(crate) fn update_preparing(job_id: Option<&str>, percent: u8, message: &str) {
    let Some(job_id) = job_id else { return };
    update_progress(job_id, |progress| {
        progress.stage = "preparing".into();
        progress.percent = percent.min(99);
        progress.message = message.into();
    });
}

pub(crate) fn finish_progress(job_id: Option<&str>) {
    let Some(job_id) = job_id else { return };
    update_progress(job_id, |progress| {
        progress.stage = "complete".into();
        progress.percent = 100;
        progress.message = "视频已生成，正在接收文件…".into();
        progress.done = true;
        progress.error = None;
    });
}

pub(crate) fn fail_progress(job_id: Option<&str>, error: &str) {
    let Some(job_id) = job_id else { return };
    update_progress(job_id, |progress| {
        progress.stage = "failed".into();
        progress.message = "视频导出失败".into();
        progress.done = true;
        progress.error = Some(error.to_string());
    });
}

pub(crate) fn progress(job_id: &str) -> Option<ExportProgress> {
    if !valid_job_id(job_id) {
        return None;
    }
    with_progress_store(|store| {
        let tracked = store.get_mut(job_id)?;
        tracked.progress.elapsed_ms =
            tracked.started.elapsed().as_millis().min(u64::MAX as u128) as u64;
        Some(tracked.progress.clone())
    })
}

fn update_progress(job_id: &str, callback: impl FnOnce(&mut ExportProgress)) {
    if !valid_job_id(job_id) {
        return;
    }
    with_progress_store(|store| {
        if let Some(tracked) = store.get_mut(job_id) {
            callback(&mut tracked.progress);
            tracked.progress.elapsed_ms =
                tracked.started.elapsed().as_millis().min(u64::MAX as u128) as u64;
            tracked.updated = Instant::now();
        }
    });
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum ExportFormat {
    Pdf,
    ImagesZip,
    Video,
    VideoFast,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct VideoExportOptions {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) fps: u32,
    pub(crate) profile: String,
    pub(crate) muted: bool,
}

impl VideoExportOptions {
    pub(crate) fn defaults(format: ExportFormat) -> Self {
        Self {
            width: 1920,
            height: 1080,
            fps: 30,
            muted: false,
            profile: if matches!(format, ExportFormat::VideoFast) {
                "fast"
            } else {
                "quality"
            }
            .into(),
        }
    }

    pub(crate) fn validate(self) -> Result<Self, String> {
        if !(640..=3840).contains(&self.width)
            || !(360..=2160).contains(&self.height)
            || self.width % 2 != 0
            || self.height % 2 != 0
        {
            return Err("video resolution must be even and between 640x360 and 3840x2160".into());
        }
        if !matches!(self.fps, 24 | 30 | 60) {
            return Err("video frame rate must be 24, 30, or 60 FPS".into());
        }
        if !matches!(self.profile.as_str(), "fast" | "balanced" | "quality") {
            return Err("video profile must be fast, balanced, or quality".into());
        }
        Ok(self)
    }

    pub(crate) fn is_default_for(&self, format: ExportFormat) -> bool {
        self == &Self::defaults(format)
    }
}

impl ExportFormat {
    fn renderer_name(self) -> &'static str {
        match self {
            Self::Pdf | Self::ImagesZip => "chromium",
            Self::Video | Self::VideoFast => "chromium-ffmpeg",
        }
    }

    fn helper_name(self) -> &'static str {
        match self {
            Self::Pdf => "pdf",
            Self::ImagesZip => "images",
            Self::Video | Self::VideoFast => "video",
        }
    }

    fn output_name(self) -> &'static str {
        match self {
            Self::Pdf => "presentation.pdf",
            Self::ImagesZip => "slides",
            Self::Video | Self::VideoFast => "presentation.mp4",
        }
    }
}

pub(crate) struct ExportedArtifact {
    workspace: ExportWorkspace,
    path: PathBuf,
    filename: &'static str,
    mime_type: &'static str,
    renderer: &'static str,
}

impl ExportedArtifact {
    pub(crate) fn open(&self) -> Result<File, String> {
        File::open(&self.path).map_err(|error| error.to_string())
    }

    pub(crate) fn len(&self) -> Result<u64, String> {
        fs::metadata(&self.path)
            .map(|metadata| metadata.len())
            .map_err(|error| error.to_string())
    }

    pub(crate) fn filename(&self) -> &'static str {
        self.filename
    }

    pub(crate) fn mime_type(&self) -> &'static str {
        self.mime_type
    }

    pub(crate) fn renderer(&self) -> &'static str {
        self.renderer
    }

    pub(crate) fn read_arc(&self) -> Result<std::sync::Arc<[u8]>, String> {
        fs::read(&self.path)
            .map(std::sync::Arc::from)
            .map_err(|error| error.to_string())
    }

    #[allow(dead_code)]
    pub(crate) fn workspace_path(&self) -> &Path {
        &self.workspace.root
    }
}

struct ExportWorkspace {
    root: PathBuf,
}

/// Render the actual native package without a conversion/re-save round trip.
/// Fixed helper and generated filenames only; no client-controlled shell/path.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct NativeQualityRuntime { node_executable: PathBuf, node_modules: PathBuf }

fn quality_runtime(web_root: &Path) -> Result<NativeQualityRuntime, String> {
    let runtime = if let (Some(node), Some(modules)) = (std::env::var_os("UNIPPT_NODE_PATH"), std::env::var_os("UNIPPT_PRESENTATION_NODE_MODULES")) {
        NativeQualityRuntime { node_executable: node.into(), node_modules: modules.into() }
    } else {
        let config = web_root.parent().ok_or("Missing application root")?.join(".unippt-quality-runtime.json");
        serde_json::from_slice(&fs::read(config).map_err(|_| "Native PPTX quality renderer is not configured")?).map_err(|_| "Invalid quality runtime configuration")?
    };
    if !runtime.node_executable.is_absolute() || !runtime.node_executable.is_file()
        || !runtime.node_modules.is_absolute() || !runtime.node_modules.join("@oai/artifact-tool").exists() {
        return Err("Configured native quality runtime is unavailable".into());
    }
    Ok(runtime)
}

pub(crate) fn native_quality_available(web_root: &Path) -> bool {
    if std::env::var("UNIPPT_QUALITY_RENDERER").as_deref() == Ok("libreoffice") {
        return ["/usr/bin/soffice", "/usr/bin/pdftoppm", "/usr/bin/timeout"].iter().all(|p| Path::new(p).is_file());
    }
    quality_runtime(web_root).is_ok()
}

pub(crate) fn native_quality_renderer_name() -> &'static str {
    if std::env::var("UNIPPT_QUALITY_RENDERER").as_deref() == Ok("libreoffice") {
        "libreoffice-pdf-poppler"
    } else {
        "native-pptx-artifact-tool"
    }
}

fn render_libreoffice_quality(bytes: &[u8]) -> Result<(Vec<u8>, serde_json::Value), String> {
    let workspace = ExportWorkspace::create()?;
    let input = workspace.root.join("candidate.pptx");
    fs::write(&input, bytes).map_err(|e| e.to_string())?;
    let started = Instant::now();
    let profile = workspace.root.join("lo-profile").to_string_lossy().bytes().map(|b| {
        if b.is_ascii_alphanumeric() || b"/-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }
    }).collect::<String>();
    // timeout owns the process group, including LibreOffice's wrapper child.
    // No shell, user-provided path, external URL or re-save of the source PPTX.
    let status = Command::new("/usr/bin/timeout")
        .args(["--kill-after=3s", "40s", "/usr/bin/soffice"])
        .arg(format!("-env:UserInstallation=file://{profile}"))
        .args(["--headless", "--nologo", "--nodefault", "--norestore", "--convert-to", "pdf:impress_pdf_Export", "--outdir"])
        .arg(&workspace.root).arg(&input).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
        .status().map_err(|_| "LibreOffice renderer could not start")?;
    let imported_ms = started.elapsed().as_secs_f64() * 1000.0;
    let pdf = workspace.root.join("candidate.pdf");
    if !status.success() || !pdf.is_file() { return Err("LibreOffice native PPTX rendering failed or timed out".into()); }
    let output = workspace.root.join("render");
    let status = Command::new("/usr/bin/timeout")
        .args(["--kill-after=2s", "10s", "/usr/bin/pdftoppm", "-f", "1", "-singlefile", "-r", "96", "-png"])
        .arg(&pdf).arg(&output).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
        .status().map_err(|_| "Native PDF rasterizer could not start")?;
    if !status.success() { return Err("Native PDF rasterization failed or timed out".into()); }
    let png = fs::read(output.with_extension("png")).map_err(|_| "Native PNG output missing")?;
    if png.len() > 32 * 1024 * 1024 || !png.starts_with(b"\x89PNG\r\n\x1a\n") { return Err("Invalid native quality PNG output".into()); }
    Ok((png, serde_json::json!({"renderer":"libreoffice-pdf-poppler", "importMs":imported_ms,
        "drawAndWriteMs":started.elapsed().as_secs_f64()*1000.0-imported_ms,
        "workerRoundTripMs":started.elapsed().as_secs_f64()*1000.0,"workerReused":false,
        "sourcePackageUnchanged":true,"visualFidelityPassed":false})))
}

static QUALITY_WARMUP: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
pub(crate) fn native_quality_warmup_status() -> &'static str {
    match QUALITY_WARMUP.load(Ordering::Acquire) { 1 => "warming", 2 => "completed", 3 => "failed", _ => "not_requested" }
}
pub(crate) fn prewarm_native_quality(bytes: Vec<u8>, web_root: PathBuf) -> &'static str {
    if QUALITY_WARMUP.compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire).is_ok() {
        std::thread::spawn(move || {
            // Fixed throwaway slide; no browser document, remote model or mutation.
            // Share the worker lock so an early real render queues behind warmup
            // instead of failing with a transient busy error.
            let ok = render_native_quality_inner(&bytes, &web_root, true).is_ok();
            QUALITY_WARMUP.store(if ok { 2 } else { 3 }, Ordering::Release);
        });
    }
    native_quality_warmup_status()
}
pub(crate) fn render_native_quality(bytes: &[u8], web_root: &Path) -> Result<(Vec<u8>, serde_json::Value), String> {
    render_native_quality_inner(bytes, web_root, false)
}
fn render_native_quality_inner(bytes: &[u8], web_root: &Path, warmup: bool) -> Result<(Vec<u8>, serde_json::Value), String> {
    static QUALITY_BUSY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    use std::sync::atomic::Ordering;
    if !warmup && QUALITY_BUSY.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).is_err() {
        return Err("native quality renderer busy; retry after the current render".into());
    }
    struct Release;
    impl Drop for Release { fn drop(&mut self) { QUALITY_BUSY.store(false, Ordering::Release); } }
    let _release = if warmup { None } else { Some(Release) };
    if std::env::var("UNIPPT_QUALITY_RENDERER").as_deref() == Ok("libreoffice") {
        return render_libreoffice_quality(bytes);
    }
    let runtime = quality_runtime(web_root)?;
    let workspace = ExportWorkspace::create()?;
    let input = workspace.root.join("candidate.pptx");
    let output = workspace.root.join("render.png");
    let report_path = workspace.root.join("render.png.json");
    fs::write(&input, bytes).map_err(|e| e.to_string())?;
    let helper = web_root.parent().ok_or("Missing application root")?.join("tools/pptx_quality_renderer.mjs");
    if !helper.is_file() { return Err("Native quality helper missing".into()); }
    static WORKER: OnceLock<Mutex<Option<Child>>> = OnceLock::new();
    let queue_started = Instant::now();
    let mut worker = WORKER.get_or_init(|| Mutex::new(None)).lock().map_err(|_| "Quality worker lock poisoned")?;
    let queue_ms = queue_started.elapsed().as_secs_f64() * 1000.0;
    if let Some(child) = worker.as_mut() {
        if child.try_wait().map_err(|e| e.to_string())?.is_some() { *worker = None; }
    }
    let reused = worker.is_some();
    if worker.is_none() {
        let mut command = Command::new(runtime.node_executable);
        command.env("UNIPPT_PRESENTATION_NODE_MODULES", runtime.node_modules);
        command.arg(node_cli_path(&helper)).arg("--worker").stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null());
        #[cfg(windows)] { use std::os::windows::process::CommandExt; command.creation_flags(0x08000000); }
        *worker = Some(command.spawn().map_err(|e| format!("Native quality renderer startup: {e}"))?);
    }
    let job = serde_json::json!({"input":node_cli_path(&input),"output":node_cli_path(&output)}).to_string();
    if let Err(error) = writeln!(worker.as_mut().unwrap().stdin.as_mut().ok_or("Quality worker has no stdin")?, "{job}") {
        if let Some(mut child) = worker.take() { let _ = child.kill(); let _ = child.wait(); }
        return Err(error.to_string());
    }
    let started = Instant::now();
    loop {
        if report_path.is_file() { break; }
        match worker.as_mut().unwrap().try_wait().map_err(|e| e.to_string())? {
            Some(status) => { *worker = None; return Err(format!("Native quality worker exited: {status}")); },
            None if started.elapsed() > Duration::from_secs(55) => {
                if let Some(mut child) = worker.take() { let _ = child.kill(); let _ = child.wait(); }
                return Err("Native quality rendering timed out after 55 seconds".into());
            }
            None => std::thread::sleep(Duration::from_millis(50)),
        }
    }
    let report: serde_json::Value = serde_json::from_slice(&fs::read(report_path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    if report["ok"] != true { return Err(report["error"].as_str().unwrap_or("Native rendering failed").to_string()); }
    let png = fs::read(output).map_err(|e| e.to_string())?;
    if png.len() > 32 * 1024 * 1024 || !png.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err("Invalid native quality PNG output".into());
    }
    let mut timings = report["timings"].clone();
    timings["workerReused"] = reused.into();
    timings["workerQueueMs"] = serde_json::json!(queue_ms);
    timings["workerRoundTripMs"] = serde_json::json!(started.elapsed().as_secs_f64() * 1000.0);
    Ok((png, timings))
}

fn node_cli_path(path: &Path) -> PathBuf {
    #[cfg(windows)] {
        let value = path.to_string_lossy();
        if let Some(tail) = value.strip_prefix(r"\\?\UNC\") { return PathBuf::from(format!(r"\\{tail}")); }
        if let Some(tail) = value.strip_prefix(r"\\?\") { return PathBuf::from(tail); }
    }
    path.to_path_buf()
}

impl ExportWorkspace {
    fn create() -> Result<Self, String> {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "unippt-render-{}-{timestamp}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&root).map_err(|error| error.to_string())?;
        Ok(Self { root })
    }
}

impl Drop for ExportWorkspace {
    fn drop(&mut self) {
        if self.root.starts_with(std::env::temp_dir())
            && self
                .root
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("unippt-render-"))
        {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}

fn renderer_daemon_enabled() -> bool {
    std::env::var("UNIPPT_RENDER_DAEMON")
        .ok()
        .is_none_or(|value| {
            !matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "0" | "false" | "off"
            )
        })
}

fn renderer_daemon_store() -> &'static Mutex<Option<RendererDaemon>> {
    RENDER_DAEMON.get_or_init(|| Mutex::new(None))
}

fn ensure_renderer_daemon(script: &Path, node: &std::ffi::OsStr) -> Result<String, String> {
    let mut store = renderer_daemon_store()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(daemon) = store.as_mut() {
        match daemon.child.try_wait() {
            Ok(None) => return Ok(daemon.endpoint.clone()),
            Ok(Some(_)) | Err(_) => *store = None,
        }
    }
    let mut command = Command::new(node);
    command
        .arg(script)
        .arg("--serve")
        .arg("true")
        .arg("--parentPid")
        .arg(std::process::id().to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("failed to start persistent HTML renderer: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "persistent HTML renderer stdout is unavailable".to_string())?;
    let mut line = String::new();
    if let Err(error) = BufReader::new(stdout).read_line(&mut line) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(format!(
            "failed to read persistent renderer startup: {error}"
        ));
    }
    let port = line
        .trim()
        .strip_prefix("UNIPPT_RENDER_DAEMON ")
        .and_then(|value| value.parse::<u16>().ok())
        .filter(|value| *value > 0)
        .ok_or_else(|| {
            let _ = child.kill();
            let _ = child.wait();
            format!(
                "persistent renderer returned an invalid startup line: {}",
                line.trim()
            )
        })?;
    let endpoint = format!("http://127.0.0.1:{port}/render");
    *store = Some(RendererDaemon {
        child,
        endpoint: endpoint.clone(),
    });
    Ok(endpoint)
}

fn invalidate_renderer_daemon() {
    let mut store = renderer_daemon_store()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(mut daemon) = store.take() {
        let _ = daemon.child.kill();
        let _ = daemon.child.wait();
    }
}

#[allow(clippy::too_many_arguments)]
fn run_renderer_via_daemon(
    script: &Path,
    node: &std::ffi::OsStr,
    workspace: &Path,
    input: &Path,
    output: &Path,
    format: ExportFormat,
    job_id: Option<&str>,
    options: &VideoExportOptions,
) -> Result<(), DaemonRunError> {
    let endpoint = ensure_renderer_daemon(script, node).map_err(DaemonRunError::Unavailable)?;
    let payload = serde_json::json!({
        "format": format.helper_name(),
        "input": input,
        "output": output,
        "workspace": workspace,
        "width": options.width,
        "height": options.height,
        "fps": options.fps,
        "profile": options.profile,
        "muted": options.muted,
    });
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(5))
        .timeout_write(Duration::from_secs(15))
        .timeout_read(Duration::from_secs(60 * 60))
        .build();
    let response = agent
        .post(&endpoint)
        .set("Content-Type", "application/json")
        .send_bytes(payload.to_string().as_bytes())
        .map_err(|error| DaemonRunError::Unavailable(error.to_string()))?;
    let mut diagnostics = String::new();
    let mut completed = false;
    for line in BufReader::new(response.into_reader()).lines() {
        let line = line.map_err(|error| DaemonRunError::Unavailable(error.to_string()))?;
        if let Some(payload) = line.strip_prefix("UNIPPT_PROGRESS ") {
            apply_renderer_progress(job_id, payload);
        } else if let Some(payload) = line.strip_prefix("UNIPPT_DIAGNOSTIC ") {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) {
                if let Some(message) = value.get("message").and_then(serde_json::Value::as_str) {
                    diagnostics.push_str(message);
                    diagnostics.push('\n');
                }
            }
        } else if line.starts_with("UNIPPT_DONE ") {
            completed = true;
        } else if let Some(payload) = line.strip_prefix("UNIPPT_ERROR ") {
            let message = serde_json::from_str::<serde_json::Value>(payload)
                .ok()
                .and_then(|value| {
                    value
                        .get("message")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned)
                })
                .unwrap_or_else(|| payload.to_string());
            return Err(DaemonRunError::Render(message));
        }
        if diagnostics.len() > 16 * 1024 {
            diagnostics.drain(..diagnostics.len() - 16 * 1024);
        }
    }
    if completed {
        Ok(())
    } else {
        Err(DaemonRunError::Unavailable(format!(
            "persistent renderer closed before completion: {}",
            diagnostics.trim()
        )))
    }
}

#[allow(dead_code)]
pub(crate) fn export(html: &[u8], format: ExportFormat) -> Result<ExportedArtifact, String> {
    export_tracked_with_options(html, format, None, None)
}

pub(crate) fn export_tracked_with_options(
    html: &[u8],
    format: ExportFormat,
    job_id: Option<&str>,
    video_options: Option<&VideoExportOptions>,
) -> Result<ExportedArtifact, String> {
    if !html.starts_with(b"<!doctype html>") && !html.starts_with(b"<!DOCTYPE html>") {
        return Err("renderer input is not a standalone UniPPT HTML document".into());
    }
    let workspace = ExportWorkspace::create()?;
    let input = workspace.root.join("presentation.html");
    fs::write(&input, html).map_err(|error| error.to_string())?;
    let raw_output = workspace.root.join(format.output_name());
    update_preparing(job_id, 7, "场景已准备，正在启动渲染器…");
    run_renderer(
        &workspace.root,
        &input,
        &raw_output,
        format,
        job_id,
        video_options,
    )?;
    update_preparing(job_id, 97, "渲染完成，正在封装导出文件…");

    let (path, filename, mime_type) = match format {
        ExportFormat::Pdf => (raw_output, "unippt-export.pdf", "application/pdf"),
        ExportFormat::Video | ExportFormat::VideoFast => {
            (raw_output, "unippt-export.mp4", "video/mp4")
        }
        ExportFormat::ImagesZip => {
            let archive = workspace.root.join("unippt-slides.zip");
            zip_slide_images(&raw_output, &archive)?;
            (archive, "unippt-slides.zip", "application/zip")
        }
    };
    let size = fs::metadata(&path)
        .map_err(|error| error.to_string())?
        .len();
    if size == 0 {
        return Err("Chromium renderer produced an empty artifact".into());
    }
    Ok(ExportedArtifact {
        workspace,
        path,
        filename,
        mime_type,
        renderer: format.renderer_name(),
    })
}

fn run_renderer(
    workspace: &Path,
    input: &Path,
    output: &Path,
    format: ExportFormat,
    job_id: Option<&str>,
    video_options: Option<&VideoExportOptions>,
) -> Result<(), String> {
    let script = crate::runtime_paths::file("tools/html-video-renderer/render.mjs")
        .ok_or("HTML renderer helper is missing from the running release")?;
    if !script.is_file() {
        return Err(format!(
            "HTML renderer helper is missing: {}",
            script.display()
        ));
    }
    let node = std::env::var_os("UNIPPT_NODE_PATH").unwrap_or_else(|| "node".into());
    let options = video_options
        .cloned()
        .unwrap_or_else(|| VideoExportOptions::defaults(format))
        .validate()?;
    let mut daemon_warning = None;
    if renderer_daemon_enabled() {
        match run_renderer_via_daemon(
            &script, &node, workspace, input, output, format, job_id, &options,
        ) {
            Ok(()) => return Ok(()),
            Err(DaemonRunError::Render(error)) => {
                return Err(format!("HTML renderer failed: {error}"))
            }
            Err(DaemonRunError::Unavailable(error)) => {
                invalidate_renderer_daemon();
                daemon_warning = Some(error);
            }
        }
    }
    let mut command = Command::new(node);
    command
        .arg(&script)
        .arg("--format")
        .arg(format.helper_name())
        .arg("--input")
        .arg(input)
        .arg("--output")
        .arg(output)
        .arg("--workspace")
        .arg(workspace)
        .arg("--width")
        .arg(options.width.to_string())
        .arg("--height")
        .arg(options.height.to_string())
        .arg("--fps")
        .arg(options.fps.to_string());
    if matches!(format, ExportFormat::Video | ExportFormat::VideoFast) {
        command
            .arg("--profile")
            .arg(&options.profile)
            .arg("--muted")
            .arg(if options.muted { "true" } else { "false" });
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("failed to start HTML renderer: {error}"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "failed to capture HTML renderer progress".to_string())?;
    let mut diagnostics = daemon_warning
        .map(|warning| {
            format!("persistent renderer unavailable; used one-shot fallback: {warning}\n")
        })
        .unwrap_or_default();
    for line in BufReader::new(stderr).lines() {
        let line = line.map_err(|error| error.to_string())?;
        if let Some(payload) = line.strip_prefix("UNIPPT_PROGRESS ") {
            apply_renderer_progress(job_id, payload);
        } else {
            diagnostics.push_str(&line);
            diagnostics.push('\n');
            if diagnostics.len() > 16 * 1024 {
                diagnostics.drain(..diagnostics.len() - 16 * 1024);
            }
        }
    }
    let status = child.wait().map_err(|error| error.to_string())?;
    if status.success() {
        return Ok(());
    }
    Err(format!(
        "HTML renderer failed ({}): {}",
        status,
        diagnostics.trim()
    ))
}

fn apply_renderer_progress(job_id: Option<&str>, payload: &str) {
    let Some(job_id) = job_id else { return };
    let Ok(event) = serde_json::from_str::<serde_json::Value>(payload) else {
        return;
    };
    update_progress(job_id, |progress| {
        let phase = event
            .get("phase")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("rendering");
        let total = event
            .get("totalSlides")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(progress.total_slides);
        let completed = event
            .get("completedSlides")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(progress.completed_slides);
        let cached = event
            .get("cachedSlides")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(progress.cached_slides);
        let frames = event
            .get("frames")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(progress.frames);
        progress.total_slides = total;
        progress.completed_slides = if total > 0 {
            completed.min(total)
        } else {
            completed
        };
        progress.cached_slides = cached;
        progress.frames = frames;
        progress.stage = phase.to_string();
        match phase {
            "preparing" => {
                progress.percent = progress.percent.max(8);
                progress.message = if total > 0 {
                    format!("已读取演示文稿，共 {total} 页")
                } else {
                    "正在读取演示文稿时间线…".into()
                };
            }
            "cache" | "rendering" => {
                let ratio = if total > 0 {
                    completed as f64 / total as f64
                } else {
                    0.0
                };
                progress.percent = (10.0 + ratio * 80.0).round().clamp(10.0, 90.0) as u8;
                let current = event
                    .get("currentSlide")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
                progress.message = if completed >= total && total > 0 {
                    format!("全部 {total} 页已就绪")
                } else if current > 0 {
                    format!("正在渲染第 {current}/{total} 页 · 已生成 {frames} 帧")
                } else if cached > 0 {
                    format!("已复用 {cached}/{total} 页缓存")
                } else {
                    format!("正在渲染动画 · {completed}/{total} 页")
                };
            }
            "encoding" => {
                progress.percent = 93;
                progress.message = "页面渲染完成，正在合并并封装 MP4…".into();
            }
            "audio" => {
                progress.percent = 95;
                let tracks = event
                    .get("tracks")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
                progress.message = format!("正在混合演示音频 · {tracks} 条音轨");
            }
            "rendered" => {
                progress.percent = 96;
                progress.message = format!("视频帧已完成 · 共 {frames} 帧");
            }
            _ => {}
        }
    });
}

fn zip_slide_images(directory: &Path, output: &Path) -> Result<(), String> {
    if !directory.is_dir() {
        return Err("Chromium renderer did not produce a slide image directory".into());
    }
    let mut entries = fs::read_dir(directory)
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    entries.sort_by_key(|entry| entry.file_name());
    let file = File::create(output).map_err(|error| error.to_string())?;
    let mut writer = ZipWriter::new(file);
    for entry in entries {
        if !entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_file()
        {
            continue;
        }
        let name = entry.file_name().to_string_lossy().replace('\\', "/");
        let compression = if name.ends_with(".png") {
            CompressionMethod::Stored
        } else {
            CompressionMethod::Deflated
        };
        writer
            .start_file(
                name,
                SimpleFileOptions::default().compression_method(compression),
            )
            .map_err(|error| error.to_string())?;
        let mut source = File::open(entry.path()).map_err(|error| error.to_string())?;
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let read = source
                .read(&mut buffer)
                .map_err(|error| error.to_string())?;
            if read == 0 {
                break;
            }
            writer
                .write_all(&buffer[..read])
                .map_err(|error| error.to_string())?;
        }
    }
    writer.finish().map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset_transport::AssetCatalog;
    use crate::{lossless_html, opc_snapshot};

    #[test]
    fn rejects_non_html_input_before_launching_renderer() {
        let error = match export(b"PK\x03\x04", ExportFormat::Pdf) {
            Ok(_) => panic!("non-HTML renderer input was accepted"),
            Err(error) => error,
        };
        assert!(error.contains("not a standalone"));
    }

    #[test]
    fn image_zip_contains_sorted_pngs_and_manifest() {
        let workspace = ExportWorkspace::create().unwrap();
        let images = workspace.root.join("images");
        fs::create_dir(&images).unwrap();
        fs::write(images.join("slide-02.png"), b"two").unwrap();
        fs::write(images.join("slide-01.png"), b"one").unwrap();
        fs::write(images.join("manifest.json"), b"{}").unwrap();
        let archive = workspace.root.join("slides.zip");
        zip_slide_images(&images, &archive).unwrap();
        let reader = File::open(archive).unwrap();
        let mut zip = zip::ZipArchive::new(reader).unwrap();
        assert_eq!(zip.by_index(0).unwrap().name(), "manifest.json");
        assert_eq!(zip.by_index(1).unwrap().name(), "slide-01.png");
        assert_eq!(zip.by_index(2).unwrap().name(), "slide-02.png");
    }

    #[test]
    fn validates_video_quality_resolution_and_frame_rate() {
        let options = VideoExportOptions {
            width: 3840,
            height: 2160,
            fps: 60,
            profile: "quality".into(),
            muted: false,
        }
        .validate()
        .unwrap();
        assert_eq!(options.width, 3840);
        assert!(!VideoExportOptions::defaults(ExportFormat::Video).muted);
        assert!(VideoExportOptions {
            width: 1920,
            height: 1080,
            fps: 29,
            profile: "quality".into(),
            muted: false,
        }
        .validate()
        .is_err());
        assert!(VideoExportOptions {
            width: 1920,
            height: 1080,
            fps: 30,
            profile: "unknown".into(),
            muted: false,
        }
        .validate()
        .is_err());
    }

    #[test]
    fn renderer_events_update_pollable_progress() {
        let job_id = "progress-test-renderer-events";
        begin_progress(job_id, ExportFormat::Video);
        apply_renderer_progress(
            Some(job_id),
            r#"{"phase":"rendering","totalSlides":12,"completedSlides":3,"cachedSlides":2,"currentSlide":4,"frames":180}"#,
        );
        let snapshot = progress(job_id).unwrap();
        assert_eq!(snapshot.stage, "rendering");
        assert_eq!(snapshot.total_slides, 12);
        assert_eq!(snapshot.completed_slides, 3);
        assert_eq!(snapshot.cached_slides, 2);
        assert_eq!(snapshot.frames, 180);
        assert_eq!(snapshot.percent, 30);
        assert!(snapshot.message.contains("4/12"));
        finish_progress(Some(job_id));
        let completed = progress(job_id).unwrap();
        assert!(completed.done);
        assert_eq!(completed.percent, 100);
    }

    #[test]
    #[ignore = "set UNIPPT_RENDER_PPTX and run explicitly"]
    fn real_pptx_renders_without_office() {
        let source = PathBuf::from(
            std::env::var_os("UNIPPT_RENDER_PPTX")
                .expect("UNIPPT_RENDER_PPTX must point to a real PPTX"),
        );
        let bytes = fs::read(&source).unwrap();
        let imported = unippt_core::import_pptx_compact(&bytes).unwrap();
        let assets = AssetCatalog::from_imported_assets(imported.assets).unwrap();
        let opc = opc_snapshot::explode(&bytes).unwrap();
        let html = lossless_html::encode_cached(
            &imported.deck,
            &imported.deck,
            "render-qa",
            &assets,
            &opc,
        )
        .unwrap();
        let format = match std::env::var("UNIPPT_RENDER_FORMAT").as_deref() {
            Ok("pdf") => ExportFormat::Pdf,
            Ok("video") => ExportFormat::Video,
            _ => ExportFormat::ImagesZip,
        };
        let artifact = export(&html, format).unwrap();
        assert!(artifact.len().unwrap() > 1024);
        if let Some(destination) = std::env::var_os("UNIPPT_RENDER_OUTPUT") {
            fs::copy(&artifact.path, destination).unwrap();
        }
    }
}
