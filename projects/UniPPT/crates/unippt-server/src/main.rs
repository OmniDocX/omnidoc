use std::collections::HashMap;
use std::fs;
use std::io::{Cursor, Read};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex, MutexGuard};

use serde::Deserialize;
use sha2::{Digest, Sha256};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};
use unippt_core::{export_pptx, export_pptx_with_baseline, import_pptx_compact, Deck};

mod ai_proxy;
mod asset_transport;
mod document_cache;
mod lossless_html;
mod math;
mod runtime_paths;
mod media_derivative;
mod opc_snapshot;
mod render_export;
mod udoc;

use asset_transport::{
    externalize_deck_assets, materialize_changed_deck_assets, materialize_deck_assets, AssetCatalog,
};
use document_cache::{ArtifactKind, CachedDocument, DocumentCache};
use opc_snapshot::OpcSnapshot;

const MAX_UPLOAD_BYTES: u64 = 128 * 1024 * 1024;
const MAX_EXPORT_JSON_BYTES: u64 = 256 * 1024 * 1024;
const MAX_VECTOR_BYTES: u64 = 64 * 1024 * 1024;
const MAX_AI_BODY_BYTES: u64 = 32 * 1024 * 1024;
const MAX_AI_CONFIG_BYTES: u64 = 64 * 1024;
type SharedDocumentCache = Arc<Mutex<DocumentCache>>;

struct StaticSite {
    root: PathBuf,
    index_html: Arc<[u8]>,
    brotli_cache: Mutex<HashMap<String, Arc<[u8]>>>,
}

impl StaticSite {
    fn load(root: PathBuf) -> std::io::Result<Self> {
        let template = fs::read(root.join("index.html"))?;
        let index_html = Arc::from(render_versioned_index(&root, &template)?);
        Ok(Self {
            root,
            index_html,
            brotli_cache: Mutex::new(HashMap::new()),
        })
    }

    fn brotli_bytes(&self, digest: &str, bytes: &[u8]) -> std::io::Result<Arc<[u8]>> {
        if let Some(cached) = self
            .brotli_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(digest)
            .cloned()
        {
            return Ok(cached);
        }

        let mut input = Cursor::new(bytes);
        let mut output = Vec::with_capacity(bytes.len() / 3);
        // Quality 5 is intentionally chosen for first-request latency. Static
        // content hashes make the result reusable for the lifetime of the
        // process, so hot requests do not recompress the same asset.
        let params = brotli::enc::BrotliEncoderParams {
            quality: 5,
            lgwin: 20,
            ..Default::default()
        };
        brotli::BrotliCompress(&mut input, &mut output, &params)?;
        let compressed: Arc<[u8]> = Arc::from(output);
        let mut cache = self
            .brotli_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if cache.len() >= 128 {
            cache.clear();
        }
        Ok(cache
            .entry(digest.to_owned())
            .or_insert_with(|| Arc::clone(&compressed))
            .clone())
    }
}

#[derive(Deserialize)]
struct ExportRequest {
    #[serde(default)]
    deck: Option<Deck>,
    #[serde(default, alias = "cacheId")]
    cache_id: Option<String>,
    #[serde(default, alias = "cacheRevision")]
    cache_revision: Option<u64>,
}

enum ExportInput {
    Cached(CachedDocument),
    Deck(Deck),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RequestLane {
    Interactive,
    Work,
}

fn request_lane(method: &Method) -> RequestLane {
    if matches!(method, Method::Get | Method::Head) {
        RequestLane::Interactive
    } else {
        RequestLane::Work
    }
}

fn spawn_request_workers(
    lane: &'static str,
    worker_count: usize,
    receiver: mpsc::Receiver<Request>,
    static_site: Arc<StaticSite>,
    document_cache: SharedDocumentCache,
) -> std::io::Result<()> {
    let receiver = Arc::new(Mutex::new(receiver));
    for index in 0..worker_count {
        let receiver = Arc::clone(&receiver);
        let static_site = Arc::clone(&static_site);
        let document_cache = Arc::clone(&document_cache);
        std::thread::Builder::new()
            .name(format!("unippt-{lane}-{index}"))
            .spawn(move || loop {
                let request = {
                    let receiver = receiver
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    receiver.recv()
                };
                let Ok(request) = request else { break };
                if let Err(error) = handle_request(request, &static_site, &document_cache) {
                    eprintln!("{lane} request failed: {error}");
                }
            })?;
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let port = std::env::var("UNIPPT_PORT").unwrap_or_else(|_| "8141".into());
    let address = format!("127.0.0.1:{port}");
    let static_site = Arc::new(StaticSite::load(find_web_root()?)?);
    let server = Server::http(&address)?;
    let document_cache = Arc::new(Mutex::new(DocumentCache::from_env()));

    println!("UniPPT 已启动: http://{address}");
    println!("Web 根目录: {}", static_site.root.display());

    let stats = lock_cache(&document_cache).stats();
    println!(
        "document cache: {} document(s), {} MiB",
        stats.max_documents,
        stats.max_bytes / 1024 / 1024
    );

    // Imports, exports and AI streams can occupy a request thread for minutes.
    // Keep them off the lane that serves the editor shell, `/api/demo`, health
    // checks and progress polling so opening a second tab can never be starved
    // by background work from the first one.
    let worker_count = std::env::var("UNIPPT_WORKERS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(usize::from)
                .unwrap_or(2)
                .clamp(2, 4)
        })
        .clamp(1, 16);
    let interactive_worker_count = std::env::var("UNIPPT_INTERACTIVE_WORKERS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(4)
        .clamp(2, 16);
    let (interactive_sender, interactive_receiver) = mpsc::channel();
    let (work_sender, work_receiver) = mpsc::sync_channel(worker_count.saturating_mul(2).max(1));
    spawn_request_workers(
        "interactive",
        interactive_worker_count,
        interactive_receiver,
        Arc::clone(&static_site),
        Arc::clone(&document_cache),
    )?;
    spawn_request_workers(
        "work",
        worker_count,
        work_receiver,
        Arc::clone(&static_site),
        Arc::clone(&document_cache),
    )?;
    println!("HTTP workers: {interactive_worker_count} interactive + {worker_count} work");
    for request in server.incoming_requests() {
        match request_lane(request.method()) {
            RequestLane::Interactive => {
                if interactive_sender.send(request).is_err() {
                    break;
                }
            }
            RequestLane::Work => match work_sender.try_send(request) {
                Ok(()) => {}
                Err(mpsc::TrySendError::Full(request)) => {
                    let response = Response::from_string(
                        serde_json::json!({
                            "error": "后台任务已满，请等待当前导入、导出或 AI 任务完成后重试"
                        })
                        .to_string(),
                    )
                    .with_status_code(StatusCode(503))
                    .with_header(header("Content-Type", "application/json; charset=utf-8"))
                    .with_header(header("Cache-Control", "no-store"))
                    .with_header(header("Retry-After", "2"))
                    .with_header(header("Server", "UniPPT"));
                    if let Err(error) = request.respond(response) {
                        eprintln!("busy response failed: {error}");
                    }
                }
                Err(mpsc::TrySendError::Disconnected(_)) => break,
            },
        }
    }
    Ok(())
}

fn handle_request(
    mut request: Request,
    static_site: &StaticSite,
    document_cache: &SharedDocumentCache,
) -> std::io::Result<()> {
    let path = request.url().split('?').next().unwrap_or("/").to_string();
    if request.method() == &Method::Get {
        if let Some(job_id) = path.strip_prefix("/api/export-progress/") {
            return match render_export::progress(job_id) {
                Some(progress) => match serde_json::to_string(&progress) {
                    Ok(json) => respond_json(request, StatusCode(200), &json),
                    Err(error) => respond_error(request, StatusCode(500), &error.to_string()),
                },
                None => respond_error(request, StatusCode(404), "导出任务尚未开始或已过期"),
            };
        }
        if let Some((cache_id, asset_id)) = parse_cached_asset_path(&path) {
            return serve_cached_asset(request, document_cache, cache_id, asset_id);
        }
    }
    match (request.method(), path.as_str()) {
        (&Method::Get, "/api/health") => respond_json(
            request,
            StatusCode(200),
            r#"{"ok":true,"engine":"unippt-pptx"}"#,
        ),
        (&Method::Get, "/api/ai/status") => match serde_json::to_string(&ai_proxy::status()) {
            Ok(json) => respond_json(request, StatusCode(200), &json),
            Err(error) => respond_error(request, StatusCode(500), &error.to_string()),
        },
        (&Method::Post, "/api/ai/config") => {
            let is_loopback = request
                .remote_addr()
                .is_some_and(|address| address.ip().is_loopback());
            if !is_loopback {
                return respond_error(request, StatusCode(403), "模型密钥只能从运行服务的本机保存");
            }
            let body = match read_limited(&mut request, MAX_AI_CONFIG_BYTES) {
                Ok(body) => body,
                Err((status, error)) => return respond_error(request, status, &error),
            };
            match ai_proxy::save_local_config(&body) {
                Ok(status) => match serde_json::to_string(&status) {
                    Ok(json) => respond_json(request, StatusCode(200), &json),
                    Err(error) => respond_error(request, StatusCode(500), &error.to_string()),
                },
                Err((status, error)) => respond_error(request, StatusCode(status as u16), &error),
            }
        }
        (&Method::Post, "/api/ai/chat") => {
            let body = match read_limited(&mut request, MAX_AI_BODY_BYTES) {
                Ok(body) => body,
                Err((status, error)) => return respond_error(request, status, &error),
            };
            match ai_proxy::chat_upstream(body) {
                Ok(reader) => request.respond(Response::new(
                    StatusCode(200),
                    vec![
                        header("Content-Type", "text/event-stream; charset=utf-8"),
                        header("Cache-Control", "no-store"),
                        header("X-Accel-Buffering", "no"),
                        header("Server", "UniPPT"),
                    ],
                    reader,
                    None,
                    None,
                )),
                Err((status, error)) => respond_error(request, StatusCode(status as u16), &error),
            }
        }
        (&Method::Post, "/api/ai/ocr") => {
            let body = match read_limited(&mut request, MAX_AI_BODY_BYTES) {
                Ok(body) => body,
                Err((status, error)) => return respond_error(request, status, &error),
            };
            match ai_proxy::ocr_detect(&body) {
                Ok(result) => match serde_json::to_string(&result) {
                    Ok(json) => respond_json(request, StatusCode(200), &json),
                    Err(error) => respond_error(request, StatusCode(500), &error.to_string()),
                },
                Err((status, error)) => respond_error(request, StatusCode(status as u16), &error),
            }
        }
        (&Method::Post, "/api/ai/inpaint") => {
            let body = match read_limited(&mut request, MAX_AI_BODY_BYTES) {
                Ok(body) => body,
                Err((status, error)) => return respond_error(request, status, &error),
            };
            match ai_proxy::clean_image(&body) {
                Ok(result) => match serde_json::to_string(&result) {
                    Ok(json) => respond_json(request, StatusCode(200), &json),
                    Err(error) => respond_error(request, StatusCode(500), &error.to_string()),
                },
                Err((status, error)) => respond_error(request, StatusCode(status as u16), &error),
            }
        }
        (&Method::Get, "/api/cache-stats") => {
            match serde_json::to_vec(&lock_cache(document_cache).stats()) {
                Ok(json) => respond_json_arc(request, StatusCode(200), Arc::from(json), vec![]),
                Err(error) => respond_error(request, StatusCode(500), &error.to_string()),
            }
        }
        (&Method::Get, "/api/demo") => match serde_json::to_string(&Deck::demo()) {
            Ok(json) => respond_json(request, StatusCode(200), &json),
            Err(error) => respond_error(request, StatusCode(500), &error.to_string()),
        },
        (&Method::Post, "/api/import-pptx") => {
            let requested_title = import_title(&request);
            let bytes = match read_limited(&mut request, MAX_UPLOAD_BYTES) {
                Ok(bytes) => bytes,
                Err((status, error)) => return respond_error(request, status, &error),
            };
            let digest = DocumentCache::digest(&bytes);
            if let Some(document) = lock_cache(document_cache).get_by_digest(&digest) {
                return respond_cached_deck(request, &document, "hit");
            }
            match import_pptx_with_snapshot(&bytes, requested_title) {
                Ok((deck, assets, opc)) => {
                    cache_core_imported_snapshot(request, deck, assets, opc, digest, document_cache)
                }
                Err((status, error)) => respond_error(request, status, &error),
            }
        }
        (&Method::Get, "/api/ai/reconstruction/status") => {
            respond_json(request, StatusCode(200), &serde_json::json!({"available":render_export::native_quality_available(&static_site.root),"renderer":render_export::native_quality_renderer_name()}).to_string())
        }
        (&Method::Post, "/api/mcp/prewarm") => {
            if let Err((status, error)) = read_limited(&mut request, 1024) { return respond_error(request, status, &error); }
            let mut state = render_export::native_quality_warmup_status();
            if state == "not_requested" {
                let mut deck = Deck::demo();
                deck.slides.truncate(1);
                match native_pptx(&mut deck, document_cache) {
                    Ok(bytes) => { state = render_export::prewarm_native_quality(bytes, static_site.root.clone()); },
                    Err((status, error)) => return respond_error(request, status, &error),
                }
            }
            respond_json(request, StatusCode(202), &serde_json::json!({"state":state,"blocking":false,"documentUsed":false}).to_string())
        }
        (&Method::Post, "/api/ai/reconstruction/render" | "/api/mcp/render-bundle") => {
            let bundled = request.url().split('?').next() == Some("/api/mcp/render-bundle");
            let started = std::time::Instant::now();
            let input = match read_export_input(&mut request, document_cache) {
                Ok(input) => input,
                Err((status, error)) => return respond_error(request, status, &error),
            };
            let ExportInput::Deck(mut deck) = input else {
                return respond_error(request, StatusCode(422), "Quality rendering needs an explicit isolated slide");
            };
            if deck.slides.len() != 1 || deck.width > 4096.0 || deck.height > 4096.0
                || deck.width < 1.0 || deck.height < 1.0 || deck.slides[0].objects.len() > 1000 {
                return respond_error(request, StatusCode(422), "Quality rendering requires one bounded slide (4096 px, 1000 objects maximum)");
            }
            let pptx = match native_pptx(&mut deck, document_cache) {
                Ok(bytes) => bytes,
                Err((status, error)) => return respond_error(request, status, &error),
            };
            let export_ms = started.elapsed().as_secs_f64() * 1000.0;
            let native_audit = match native_quality_audit(&pptx) {
                Ok(audit) => audit,
                Err(error) => return respond_error(request, StatusCode(422), &error),
            };
            match render_export::render_native_quality(&pptx, &static_site.root) {
                Ok((png, mut timings)) => {
                    timings["exportMs"] = serde_json::json!(export_ms);
                    timings["serverTotalMs"] = serde_json::json!(started.elapsed().as_secs_f64()*1000.0);
                    if bundled {
                        use base64::Engine;
                        let encode = |bytes: &[u8]| base64::engine::general_purpose::STANDARD.encode(bytes);
                        respond_json(request, StatusCode(200), &serde_json::json!({"pngBase64":encode(&png),"pptxBase64":encode(&pptx),
                            "audit":serde_json::from_str::<serde_json::Value>(&native_audit).unwrap_or_default(),"timings":timings,
                            "slideIds":[deck.slides[0].id],"nativeRendered":true,"visualFidelityPassed":false}).to_string())
                    } else { request.respond(Response::from_data(png).with_header(header("Content-Type", "image/png"))
                    .with_header(header("Cache-Control", "no-store"))
                    .with_header(header("X-UniPPT-Quality-Renderer", render_export::native_quality_renderer_name()))
                    .with_header(header("X-UniPPT-Native-Audit", &native_audit))
                    .with_header(header("X-UniPPT-Render-Timings", &timings.to_string()))) }
                },
                Err(error) => respond_error(request, StatusCode(503), &error),
            }
        }
        (&Method::Post, "/api/export-pptx") => {
            let input = match read_export_input(&mut request, document_cache) {
                Ok(input) => input,
                Err((status, error)) => return respond_error(request, status, &error),
            };
            match input {
                ExportInput::Cached(document) => {
                    match cached_native_pptx(document_cache, &document) {
                        Ok(pptx) => respond_pptx_arc(request, pptx, &document),
                        Err((status, error)) => respond_error(request, status, &error),
                    }
                }
                ExportInput::Deck(mut deck) => match native_pptx(&mut deck, document_cache) {
                    Ok(pptx) => respond_pptx(request, pptx),
                    Err((status, error)) => respond_error(request, status, &error),
                },
            }
        }
        (&Method::Post, "/api/export-udoc") => {
            let input = match read_export_input(&mut request, document_cache) {
                Ok(input) => input,
                Err((status, error)) => return respond_error(request, status, &error),
            };
            match input {
                ExportInput::Cached(document) => {
                    match cached_artifact(document_cache, &document, ArtifactKind::Udoc, || {
                        udoc::encode_cached(
                            &document.deck,
                            &document.original_deck,
                            &document.assets,
                            &document.opc,
                            &document.deck.title,
                        )
                        .map_err(|error| (StatusCode(422), error.to_string()))
                    }) {
                        Ok(bytes) => respond_download_arc(
                            request,
                            bytes,
                            "application/vnd.unidoc",
                            "unippt-export.udoc",
                            &document,
                        ),
                        Err((status, error)) => respond_error(request, status, &error),
                    }
                }
                ExportInput::Deck(mut deck) => {
                    match native_pptx(&mut deck, document_cache).and_then(|pptx| {
                        udoc::encode(&deck, &pptx, &deck.title)
                            .map_err(|error| (StatusCode(422), error.to_string()))
                    }) {
                        Ok(bytes) => respond_download(
                            request,
                            bytes,
                            "application/vnd.unidoc",
                            "unippt-export.udoc",
                        ),
                        Err((status, error)) => respond_error(request, status, &error),
                    }
                }
            }
        }
        (&Method::Post, "/api/udoc-json") => {
            let input = match read_export_input(&mut request, document_cache) {
                Ok(input) => input,
                Err((status, error)) => return respond_error(request, status, &error),
            };
            match input {
                ExportInput::Cached(document) => {
                    match cached_artifact(document_cache, &document, ArtifactKind::UdocJson, || {
                        serde_json::to_vec(&udoc::structure(&document.deck))
                            .map_err(|error| (StatusCode(500), error.to_string()))
                    }) {
                        Ok(json) => respond_json_arc(
                            request,
                            StatusCode(200),
                            json,
                            cache_response_headers(&document, "hit"),
                        ),
                        Err((status, error)) => respond_error(request, status, &error),
                    }
                }
                ExportInput::Deck(deck) => match serde_json::to_vec(&udoc::structure(&deck)) {
                    Ok(json) => respond_json_arc(request, StatusCode(200), Arc::from(json), vec![]),
                    Err(error) => respond_error(request, StatusCode(500), &error.to_string()),
                },
            }
        }
        (&Method::Post, "/api/export-html") => {
            let input = match read_export_input(&mut request, document_cache) {
                Ok(input) => input,
                Err((status, error)) => return respond_error(request, status, &error),
            };
            match input {
                ExportInput::Cached(document) => {
                    match cached_artifact(document_cache, &document, ArtifactKind::Html, || {
                        lossless_html::encode_cached(
                            &document.deck,
                            &document.original_deck,
                            &document.id,
                            &document.assets,
                            &document.opc,
                        )
                        .map_err(|error| (StatusCode(422), error.to_string()))
                    }) {
                        Ok(bytes) => respond_download_arc(
                            request,
                            bytes,
                            "text/html; charset=utf-8",
                            "unippt-lossless.html",
                            &document,
                        ),
                        Err((status, error)) => respond_error(request, status, &error),
                    }
                }
                ExportInput::Deck(mut deck) => {
                    match native_pptx(&mut deck, document_cache).and_then(|pptx| {
                        lossless_html::encode(&deck, &pptx)
                            .map_err(|error| (StatusCode(422), error.to_string()))
                    }) {
                        Ok(bytes) => respond_download(
                            request,
                            bytes,
                            "text/html; charset=utf-8",
                            "unippt-lossless.html",
                        ),
                        Err((status, error)) => respond_error(request, status, &error),
                    }
                }
            }
        }
        (
            &Method::Post,
            endpoint @ ("/api/export-pdf"
            | "/api/export-images"
            | "/api/export-video"
            | "/api/export-video-fast"),
        ) => {
            let format = match endpoint {
                "/api/export-pdf" => render_export::ExportFormat::Pdf,
                "/api/export-images" => render_export::ExportFormat::ImagesZip,
                "/api/export-video" => render_export::ExportFormat::Video,
                "/api/export-video-fast" => render_export::ExportFormat::VideoFast,
                _ => unreachable!(),
            };
            let job_id = export_job_id(&request);
            let video_options = match video_export_options(&request, format) {
                Ok(options) => options,
                Err(error) => {
                    render_export::fail_progress(job_id.as_deref(), &error);
                    return respond_error(request, StatusCode(400), &error);
                }
            };
            if let Some(job_id) = job_id.as_deref() {
                render_export::begin_progress(job_id, format);
            }
            let input = match read_export_input(&mut request, document_cache) {
                Ok(input) => input,
                Err((status, error)) => {
                    render_export::fail_progress(job_id.as_deref(), &error);
                    return respond_error(request, status, &error);
                }
            };
            let video_cache_kind = video_options
                .as_ref()
                .filter(|options| options.is_default_for(format))
                .and(match format {
                    render_export::ExportFormat::Video => Some(ArtifactKind::Video),
                    render_export::ExportFormat::VideoFast => Some(ArtifactKind::VideoFast),
                    _ => None,
                });
            if let (Some(kind), ExportInput::Cached(document)) = (video_cache_kind, &input) {
                let cached =
                    lock_cache(document_cache).artifact(&document.id, document.revision, kind);
                if let Some(bytes) = cached {
                    render_export::finish_progress(job_id.as_deref());
                    return respond_rendered_arc(
                        request,
                        bytes,
                        "video/mp4",
                        "unippt-export.mp4",
                        document,
                        "chromium-ffmpeg-cache",
                    );
                }
            }
            let (html, cached_document): (Arc<[u8]>, Option<CachedDocument>) = match input {
                ExportInput::Cached(document) => {
                    let html = match lossless_html::encode_render_cached(
                        &document.deck,
                        &document.id,
                        &document.assets,
                    ) {
                        Ok(html) => Arc::from(html),
                        Err(error) => {
                            render_export::fail_progress(job_id.as_deref(), &error.to_string());
                            return respond_error(request, StatusCode(422), &error.to_string());
                        }
                    };
                    (html, Some(document))
                }
                ExportInput::Deck(deck) => {
                    let html = match lossless_html::encode_render(&deck) {
                        Ok(html) => Arc::from(html),
                        Err(error) => {
                            render_export::fail_progress(job_id.as_deref(), &error.to_string());
                            return respond_error(request, StatusCode(422), &error.to_string());
                        }
                    };
                    (html, None)
                }
            };
            match render_export::export_tracked_with_options(
                &html,
                format,
                job_id.as_deref(),
                video_options.as_ref(),
            ) {
                Ok(artifact) => {
                    if let (Some(kind), Some(document)) = (video_cache_kind, &cached_document) {
                        let bytes = match artifact.read_arc() {
                            Ok(bytes) => bytes,
                            Err(error) => {
                                render_export::fail_progress(job_id.as_deref(), &error);
                                return respond_error(request, StatusCode(500), &error);
                            }
                        };
                        lock_cache(document_cache).store_artifact(
                            &document.id,
                            document.revision,
                            kind,
                            Arc::clone(&bytes),
                        );
                        render_export::finish_progress(job_id.as_deref());
                        return respond_rendered_arc(
                            request,
                            bytes,
                            artifact.mime_type(),
                            artifact.filename(),
                            document,
                            artifact.renderer(),
                        );
                    }
                    render_export::finish_progress(job_id.as_deref());
                    respond_exported_artifact(request, artifact, cached_document.as_ref())
                }
                Err(error) => {
                    render_export::fail_progress(job_id.as_deref(), &error);
                    respond_error(request, StatusCode(503), &error)
                }
            }
        }
        (&Method::Post, "/api/import-udoc") => {
            let bytes = match read_limited(&mut request, MAX_EXPORT_JSON_BYTES) {
                Ok(bytes) => bytes,
                Err((status, error)) => return respond_error(request, status, &error),
            };
            match udoc::decode_cached(&bytes) {
                Ok(decoded) => cache_imported_compact_deck(
                    request,
                    decoded.deck,
                    decoded.original_deck,
                    decoded.assets,
                    decoded.opc,
                    decoded.presentation_digest,
                    document_cache,
                ),
                Err(error) => respond_error(request, StatusCode(422), &error.to_string()),
            }
        }
        (&Method::Post, "/api/import-html") => {
            let bytes = match read_limited(&mut request, MAX_EXPORT_JSON_BYTES) {
                Ok(bytes) => bytes,
                Err((status, error)) => return respond_error(request, status, &error),
            };
            match lossless_html::decode_compact(&bytes) {
                Ok(decoded) if decoded.version == 4 => restore_snapshot_html(
                    request,
                    decoded.deck,
                    decoded.original_deck,
                    decoded.assets,
                    decoded.opc,
                    decoded.presentation_digest,
                    document_cache,
                ),
                Ok(decoded) if decoded.version == 3 => restore_compact_html(
                    request,
                    decoded.deck,
                    decoded
                        .presentation
                        .expect("legacy HTML always contains native PPTX"),
                    decoded.assets,
                    document_cache,
                ),
                Ok(decoded) => restore_portable_deck(
                    request,
                    decoded.deck,
                    decoded
                        .presentation
                        .expect("legacy HTML always contains native PPTX"),
                    document_cache,
                ),
                Err(error) => respond_error(request, StatusCode(422), &error.to_string()),
            }
        }
        (&Method::Post, "/api/math/latex-to-omml") => {
            handle_formula_request(request, "latex-to-omml")
        }
        (&Method::Post, "/api/math/omml-to-latex") => {
            handle_formula_request(request, "omml-to-latex")
        }
        (&Method::Post, "/api/vector/emf-to-svg") => handle_vector_request(request, "emf-to-svg"),
        (&Method::Post, "/api/vector/svg-to-emf") => handle_vector_request(request, "svg-to-emf"),
        (&Method::Get, _) | (&Method::Head, _) => serve_static(request, static_site, &path),
        _ => respond_error(request, StatusCode(405), "不支持的请求方法"),
    }
}

fn read_export_input(
    request: &mut Request,
    document_cache: &SharedDocumentCache,
) -> Result<ExportInput, (StatusCode, String)> {
    let bytes = read_limited(request, MAX_EXPORT_JSON_BYTES)?;
    let payload = serde_json::from_slice::<ExportRequest>(&bytes)
        .map_err(|error| (StatusCode(400), error.to_string()))?;
    if let Some(mut deck) = payload.deck {
        let cache_id = payload.cache_id.or_else(|| deck.source_import_id.clone());
        if let Some(cache_id) = cache_id {
            let current = lock_cache(document_cache).get(&cache_id).ok_or_else(|| {
                (
                    StatusCode(409),
                    "cached document expired; import the source presentation again".into(),
                )
            })?;
            let client_revision = payload
                .cache_revision
                .unwrap_or_else(|| current.revision.saturating_add(1));
            if client_revision == current.revision {
                return Ok(ExportInput::Cached(current));
            }
            if client_revision < current.revision {
                return Err((
                    StatusCode(409),
                    format!(
                        "stale browser revision {client_revision}; current revision is {}",
                        current.revision
                    ),
                ));
            }
            deck.source_import_id = Some(cache_id.clone());
            math::ensure_native_formulas(&mut deck)
                .map_err(|error| (StatusCode(422), error.to_string()))?;
            let mut assets = (*current.assets).clone();
            externalize_deck_assets(&mut deck, &cache_id, &mut assets)
                .map_err(|error| (StatusCode(422), error))?;
            let estimated_deck_bytes = serde_json::to_vec(&deck)
                .map_err(|error| (StatusCode(500), error.to_string()))?
                .len();
            let deck = Arc::new(deck);
            let assets = Arc::new(assets);
            return lock_cache(document_cache)
                .update_deck(
                    &cache_id,
                    client_revision,
                    deck,
                    estimated_deck_bytes,
                    assets,
                )
                .map(ExportInput::Cached)
                .map_err(|error| {
                    let status = if error.contains("limit is") {
                        StatusCode(507)
                    } else {
                        StatusCode(409)
                    };
                    (status, error)
                });
        }
        return Ok(ExportInput::Deck(deck));
    }
    let cache_id = payload.cache_id.ok_or_else(|| {
        (
            StatusCode(400),
            "request must contain either deck or cache_id".into(),
        )
    })?;
    let document = lock_cache(document_cache).get(&cache_id).ok_or_else(|| {
        (
            StatusCode(409),
            "cached document expired; import the source presentation again".into(),
        )
    })?;
    if let Some(client_revision) = payload.cache_revision {
        if client_revision != document.revision {
            return Err((
                StatusCode(409),
                format!(
                    "browser revision {client_revision} does not match cached revision {}",
                    document.revision
                ),
            ));
        }
    }
    Ok(ExportInput::Cached(document))
}

/// Reads a request body, mapping both overflow and transport failures onto a
/// status the caller can answer with.
///
/// Propagating the `io::Error` instead would abandon the `Request` without a
/// response, and the HTTP layer then closes the exchange with a bare status and
/// an empty body. The browser feeds that empty body to `JSON.parse` and reports
/// a parser error, hiding the real failure.
fn read_limited(request: &mut Request, limit: u64) -> Result<Vec<u8>, (StatusCode, String)> {
    let mut bytes = Vec::new();
    if let Err(error) = request.as_reader().take(limit + 1).read_to_end(&mut bytes) {
        return Err((
            StatusCode(400),
            format!("读取请求内容失败（已接收 {} 字节）：{error}", bytes.len()),
        ));
    }
    if bytes.len() as u64 > limit {
        return Err((
            StatusCode(413),
            format!("请求内容超过 {} MiB 限制", limit / 1024 / 1024),
        ));
    }
    Ok(bytes)
}

fn native_pptx(
    deck: &mut Deck,
    document_cache: &SharedDocumentCache,
) -> Result<Vec<u8>, (StatusCode, String)> {
    let source = if let Some(id) = deck.source_import_id.clone() {
        let document = lock_cache(document_cache).get(&id).ok_or_else(|| {
            (
                StatusCode(409),
                "源 PPTX 已不在缓存中，请重新导入源文件后再保存".into(),
            )
        })?;
        *deck = materialize_deck_assets(deck, &document.id, &document.assets)
            .map_err(|error| (StatusCode(422), error))?;
        Some(
            document
                .opc
                .rebuild()
                .map_err(|error| (StatusCode(422), error))?,
        )
    } else {
        None
    };
    math::ensure_native_formulas(deck).map_err(|error| (StatusCode(422), error.to_string()))?;
    export_pptx(deck, source.as_deref()).map_err(|error| (StatusCode(422), error.to_string()))
}

fn restore_portable_deck(
    request: Request,
    deck: Deck,
    presentation: Vec<u8>,
    document_cache: &SharedDocumentCache,
) -> std::io::Result<()> {
    // UDOC verifies every OPC part with SHA-256 and lossless HTML verifies
    // the decoded payload and ZIP signature. The persisted Deck is already the
    // authoritative parsed scene, so reparsing presentation.pptx here would
    // discard the main performance benefit of the portable formats.
    let digest = match portable_presentation_digest(&presentation) {
        Ok(digest) => digest,
        Err(error) => return respond_error(request, StatusCode(422), &error),
    };
    cache_imported_deck(request, deck, presentation, digest, document_cache)
}

fn native_quality_audit(bytes: &[u8]) -> Result<String, String> {
    // Inspect the canonical XML emitted by our own native writer, not a render
    // or re-import. These are structural counts, not visual acceptance.
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let mut part = archive.by_name("ppt/slides/slide1.xml").map_err(|e| e.to_string())?;
    if part.size() > 16 * 1024 * 1024 { return Err("Quality slide XML is too large".into()); }
    let mut xml = String::new(); part.read_to_string(&mut xml).map_err(|e| e.to_string())?;
    let (mut shapes, mut connectors, mut pictures, mut text_objects) = (0, 0, 0, 0);
    let (mut in_text, mut has_text) = (false, false);
    let mut reader = quick_xml::Reader::from_str(&xml);
    loop {
        use quick_xml::events::Event;
        match reader.read_event().map_err(|e| e.to_string())? {
            Event::Start(e) => match e.name().as_ref() {
                b"p:sp" => { shapes += 1; has_text = false; }
                b"p:cxnSp" => connectors += 1,
                b"p:pic" => pictures += 1,
                b"a:t" => in_text = true,
                _ => {}
            },
            Event::Text(e) if in_text => { has_text |= !e.is_empty(); }
            Event::End(e) => match e.name().as_ref() {
                b"a:t" => in_text = false,
                b"p:sp" => { if has_text { text_objects += 1; } has_text = false; }
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(serde_json::json!({"textBodies":xml.matches("<p:txBody>").count(),"nativeTextObjects":text_objects,"shapes":shapes,
        "connectors":connectors,"pictures":pictures,
        "subscriptRuns":xml.matches("baseline=\"-").count(),"rotatedTransforms":xml.matches(" rot=\"").count(),
        "transparencyElements":xml.matches("<a:alpha ").count(),"sha256":format!("{:x}",Sha256::digest(bytes))}).to_string())
}

fn portable_presentation_digest(presentation: &[u8]) -> Result<[u8; 32], String> {
    const MAX_OPC_ENTRIES: usize = 100_000;
    const MAX_OPC_EXPANDED_BYTES: u64 = 2 * 1024 * 1024 * 1024;
    const MAX_REQUIRED_PART_BYTES: u64 = 32 * 1024 * 1024;
    const REQUIRED_PARTS: [&str; 3] =
        ["[Content_Types].xml", "_rels/.rels", "ppt/presentation.xml"];

    let mut archive = zip::ZipArchive::new(Cursor::new(presentation))
        .map_err(|error| format!("便携文档中的原生 PPTX 不是有效 OPC ZIP：{error}"))?;
    if archive.len() > MAX_OPC_ENTRIES {
        return Err(format!("便携文档 OPC 部件过多：{}", archive.len()));
    }
    let mut expanded = 0u64;
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|error| format!("便携文档 OPC 中央目录损坏：{error}"))?;
        expanded = expanded
            .checked_add(entry.size())
            .ok_or_else(|| "便携文档 OPC 展开尺寸溢出".to_string())?;
        if expanded > MAX_OPC_EXPANDED_BYTES {
            return Err("便携文档 OPC 展开尺寸超过 2 GiB 安全上限".into());
        }
    }
    for part_name in REQUIRED_PARTS {
        let mut part = archive
            .by_name(part_name)
            .map_err(|_| format!("便携文档原生 PPTX 缺少 {part_name}"))?;
        if part.size() > MAX_REQUIRED_PART_BYTES {
            return Err(format!("便携文档 OPC 核心部件过大：{part_name}"));
        }
        let expected = part.size() as usize;
        let mut verified = Vec::with_capacity(expected);
        part.read_to_end(&mut verified)
            .map_err(|error| format!("便携文档 OPC 核心部件校验失败（{part_name}）：{error}"))?;
        if verified.len() != expected || verified.is_empty() {
            return Err(format!("便携文档 OPC 核心部件不完整：{part_name}"));
        }
    }
    Ok(DocumentCache::digest(presentation))
}

fn restore_compact_html(
    request: Request,
    mut deck: Deck,
    presentation: Vec<u8>,
    assets: AssetCatalog,
    document_cache: &SharedDocumentCache,
) -> std::io::Result<()> {
    let digest = match portable_presentation_digest(&presentation) {
        Ok(digest) => digest,
        Err(error) => return respond_error(request, StatusCode(422), &error),
    };
    let id = DocumentCache::id_for_digest(&digest);
    if let Err(error) = lossless_html::bind_cached_asset_refs(&mut deck, &id, &assets) {
        return respond_error(request, StatusCode(422), &error.to_string());
    }
    deck.source_import_id = Some(id);
    insert_compact_deck(request, deck, assets, presentation, digest, document_cache)
}

fn restore_snapshot_html(
    request: Request,
    mut deck: Deck,
    mut original_deck: Deck,
    assets: AssetCatalog,
    opc: OpcSnapshot,
    digest: [u8; 32],
    document_cache: &SharedDocumentCache,
) -> std::io::Result<()> {
    let id = DocumentCache::id_for_digest(&digest);
    if let Err(error) = lossless_html::bind_cached_asset_refs(&mut deck, &id, &assets) {
        return respond_error(request, StatusCode(422), &error.to_string());
    }
    if let Err(error) = lossless_html::bind_cached_asset_refs(&mut original_deck, &id, &assets) {
        return respond_error(request, StatusCode(422), &error.to_string());
    }
    deck.source_import_id = Some(id.clone());
    original_deck.source_import_id = Some(id);
    insert_snapshot_deck(
        request,
        deck,
        original_deck,
        assets,
        opc,
        digest,
        document_cache,
    )
}

fn cache_imported_deck(
    request: Request,
    mut deck: Deck,
    source: Vec<u8>,
    digest: [u8; 32],
    document_cache: &SharedDocumentCache,
) -> std::io::Result<()> {
    let id = DocumentCache::id_for_digest(&digest);
    deck.source_import_id = Some(id.clone());
    let mut assets = AssetCatalog::default();
    if let Err(error) = externalize_deck_assets(&mut deck, &id, &mut assets) {
        return respond_error(request, StatusCode(422), &error);
    }
    insert_compact_deck(request, deck, assets, source, digest, document_cache)
}

fn import_pptx_for_cache(
    bytes: &[u8],
    requested_title: Option<String>,
) -> Result<(Deck, AssetCatalog), (StatusCode, String)> {
    let mut imported =
        import_pptx_compact(bytes).map_err(|error| (StatusCode(422), error.to_string()))?;
    if let Some(title) = requested_title {
        imported.deck.title = title;
    }
    math::enrich_imported_formulas(&mut imported.deck);
    let assets = AssetCatalog::from_imported_assets(imported.assets)
        .map_err(|error| (StatusCode(422), error))?;
    let media_report =
        media_derivative::enrich_playback_assets_from_catalog(&mut imported.deck, &assets);
    if media_report.generated > 0 {
        eprintln!(
            "generated {} browser media derivative(s), assigned to {} scene object(s)",
            media_report.generated, media_report.assigned
        );
    }
    for warning in media_report.warnings {
        eprintln!("media derivative skipped: {warning}");
    }
    Ok((imported.deck, assets))
}

fn import_pptx_with_snapshot(
    bytes: &[u8],
    requested_title: Option<String>,
) -> Result<(Deck, AssetCatalog, OpcSnapshot), (StatusCode, String)> {
    // The scene importer and byte-preserving OPC snapshot both read the same
    // immutable ZIP but do not depend on each other. Running them concurrently
    // avoids paying their full expansion costs serially on first open.
    std::thread::scope(|scope| {
        let snapshot =
            scope.spawn(|| catch_unwind(AssertUnwindSafe(|| opc_snapshot::explode(bytes))));
        let imported = catch_unwind(AssertUnwindSafe(|| {
            import_pptx_for_cache(bytes, requested_title)
        }));
        let snapshot = snapshot.join().map_err(|payload| {
            let message = panic_message(payload);
            (
                StatusCode(500),
                format!("解析 PPTX 包时服务崩溃：{message}"),
            )
        })?;
        let opc = snapshot
            .map_err(|payload| {
                let message = panic_message(payload);
                (
                    StatusCode(500),
                    format!("解析 PPTX 包时服务崩溃：{message}"),
                )
            })?
            .map_err(|error| (StatusCode(422), error))?;
        let (deck, assets) = imported.map_err(|payload| {
            let message = panic_message(payload);
            (StatusCode(500), format!("导入过程中服务崩溃：{message}"))
        })??;
        Ok((deck, assets, opc))
    })
}

fn cache_core_imported_snapshot(
    request: Request,
    mut deck: Deck,
    mut assets: AssetCatalog,
    opc: OpcSnapshot,
    digest: [u8; 32],
    document_cache: &SharedDocumentCache,
) -> std::io::Result<()> {
    let id = DocumentCache::id_for_digest(&digest);
    deck.source_import_id = Some(id.clone());
    if let Err(error) = externalize_deck_assets(&mut deck, &id, &mut assets) {
        return respond_error(request, StatusCode(422), &error);
    }
    let original_deck = deck.clone();
    insert_snapshot_deck(
        request,
        deck,
        original_deck,
        assets,
        opc,
        digest,
        document_cache,
    )
}

fn cache_imported_compact_deck(
    request: Request,
    mut deck: Deck,
    mut original_deck: Deck,
    mut assets: AssetCatalog,
    opc: OpcSnapshot,
    digest: [u8; 32],
    document_cache: &SharedDocumentCache,
) -> std::io::Result<()> {
    let id = DocumentCache::id_for_digest(&digest);
    if deck.source_import_id.as_deref() != Some(id.as_str()) {
        return respond_error(
            request,
            StatusCode(422),
            "UDOC compact scene cache id does not match its native PPTX digest",
        );
    }
    if let Err(error) = externalize_deck_assets(&mut deck, &id, &mut assets) {
        return respond_error(request, StatusCode(422), &error);
    }
    if let Err(error) = externalize_deck_assets(&mut original_deck, &id, &mut assets) {
        return respond_error(request, StatusCode(422), &error);
    }
    insert_snapshot_deck(
        request,
        deck,
        original_deck,
        assets,
        opc,
        digest,
        document_cache,
    )
}

fn insert_compact_deck(
    request: Request,
    deck: Deck,
    assets: AssetCatalog,
    source: Vec<u8>,
    digest: [u8; 32],
    document_cache: &SharedDocumentCache,
) -> std::io::Result<()> {
    let opc = match catch_unwind(AssertUnwindSafe(|| opc_snapshot::explode(&source))) {
        Ok(Ok(opc)) => opc,
        Ok(Err(error)) => return respond_error(request, StatusCode(422), &error),
        Err(payload) => {
            let message = panic_message(payload);
            eprintln!("opc explode panicked: {message}");
            return respond_error(
                request,
                StatusCode(500),
                &format!("解析 PPTX 包时服务崩溃：{message}"),
            );
        }
    };
    let original_deck = deck.clone();
    insert_snapshot_deck(
        request,
        deck,
        original_deck,
        assets,
        opc,
        digest,
        document_cache,
    )
}

fn insert_snapshot_deck(
    request: Request,
    deck: Deck,
    original_deck: Deck,
    assets: AssetCatalog,
    opc: OpcSnapshot,
    digest: [u8; 32],
    document_cache: &SharedDocumentCache,
) -> std::io::Result<()> {
    let deck_json = match serde_json::to_vec(&deck) {
        Ok(json) => json,
        Err(error) => return respond_error(request, StatusCode(500), &error.to_string()),
    };
    let deck = Arc::new(deck);
    let original_deck = Arc::new(original_deck);
    let opc = Arc::new(opc);
    let deck_json: Arc<[u8]> = Arc::from(deck_json);
    let assets = Arc::new(assets);
    let inserted = lock_cache(document_cache).insert(
        digest,
        Arc::clone(&opc),
        Arc::clone(&deck),
        Arc::clone(&original_deck),
        Arc::clone(&deck_json),
        Arc::clone(&assets),
    );
    match inserted {
        Ok(document) => respond_cached_deck(request, &document, "miss"),
        Err(error) => {
            // A materialized bypass would expand every repeated cached asset
            // back into a full base64 data URI. Real decks can turn a ~20 MiB
            // unique asset set into more than 1.8 GiB of JSON, so failing the
            // bounded cache explicitly is safer than risking process OOM.
            eprintln!("document cache rejected import: {error}");
            respond_error(request, StatusCode(507), &error)
        }
    }
}

fn cached_artifact<F>(
    document_cache: &SharedDocumentCache,
    document: &CachedDocument,
    kind: ArtifactKind,
    build: F,
) -> Result<Arc<[u8]>, (StatusCode, String)>
where
    F: FnOnce() -> Result<Vec<u8>, (StatusCode, String)>,
{
    if let Some(bytes) = lock_cache(document_cache).artifact(&document.id, document.revision, kind)
    {
        return Ok(bytes);
    }
    let bytes: Arc<[u8]> = Arc::from(build()?);
    lock_cache(document_cache).store_artifact(
        &document.id,
        document.revision,
        kind,
        Arc::clone(&bytes),
    );
    Ok(bytes)
}

fn cached_native_pptx(
    document_cache: &SharedDocumentCache,
    document: &CachedDocument,
) -> Result<Arc<[u8]>, (StatusCode, String)> {
    cached_artifact(document_cache, document, ArtifactKind::Pptx, || {
        let source = document
            .opc
            .rebuild()
            .map_err(|error| (StatusCode(422), error))?;
        if !document.dirty {
            return Ok(source);
        }
        let mut deck = materialize_changed_deck_assets(
            &document.deck,
            &document.original_deck,
            &document.id,
            &document.assets,
        )
        .map_err(|error| (StatusCode(422), error))?;
        math::ensure_native_formulas(&mut deck)
            .map_err(|error| (StatusCode(422), error.to_string()))?;
        export_pptx_with_baseline(&deck, Some(&source), Some(&document.original_deck))
            .map_err(|error| (StatusCode(422), error.to_string()))
    })
}

fn lock_cache(document_cache: &SharedDocumentCache) -> MutexGuard<'_, DocumentCache> {
    document_cache
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn respond_cached_deck(
    request: Request,
    document: &CachedDocument,
    cache_status: &str,
) -> std::io::Result<()> {
    respond_json_arc(
        request,
        StatusCode(200),
        Arc::clone(&document.deck_json),
        cache_response_headers(document, cache_status),
    )
}

fn cache_response_headers(document: &CachedDocument, cache_status: &str) -> Vec<Header> {
    vec![
        header("X-UniPPT-Cache-Id", &document.id),
        header("X-UniPPT-Cache", cache_status),
        header("X-UniPPT-Cache-Revision", &document.revision.to_string()),
    ]
}

fn handle_formula_request(mut request: Request, direction: &str) -> std::io::Result<()> {
    let bytes = match read_limited(&mut request, 1024 * 1024) {
        Ok(bytes) => bytes,
        Err((status, error)) => return respond_error(request, status, &error),
    };
    let input = match serde_json::from_slice::<math::FormulaRequest>(&bytes) {
        Ok(input) => input,
        Err(error) => return respond_error(request, StatusCode(400), &error.to_string()),
    };
    match math::convert_one(direction, &input.value) {
        Ok(value) => match serde_json::to_string(&math::FormulaResponse { value }) {
            Ok(json) => respond_json(request, StatusCode(200), &json),
            Err(error) => respond_error(request, StatusCode(500), &error.to_string()),
        },
        Err(error) => respond_error(request, StatusCode(422), &error.to_string()),
    }
}

fn handle_vector_request(mut request: Request, direction: &str) -> std::io::Result<()> {
    let bytes = match read_limited(&mut request, MAX_VECTOR_BYTES) {
        Ok(bytes) => bytes,
        Err((status, error)) => return respond_error(request, status, &error),
    };
    match direction {
        "emf-to-svg" => {
            match emf2svg::emf_to_svg_with(&bytes, emf2svg::Emf2SvgOptions { lossless: true }) {
                Ok(svg) => respond_arc(
                    request,
                    StatusCode(200),
                    Arc::from(svg.into_bytes()),
                    vec![
                        header("Content-Type", "image/svg+xml; charset=utf-8"),
                        header("Cache-Control", "no-store"),
                    ],
                ),
                Err(error) => respond_error(request, StatusCode(422), &error.to_string()),
            }
        }
        "svg-to-emf" => {
            let svg = match std::str::from_utf8(&bytes) {
                Ok(svg) => svg,
                Err(error) => return respond_error(request, StatusCode(400), &error.to_string()),
            };
            let options = svg2emf::EmitOptions {
                lossless: true,
                ..svg2emf::EmitOptions::default()
            };
            match svg2emf::svg_to_emf(svg, options) {
                Ok(emf) => respond_arc(
                    request,
                    StatusCode(200),
                    Arc::from(emf),
                    vec![
                        header("Content-Type", "image/x-emf"),
                        header("Cache-Control", "no-store"),
                    ],
                ),
                Err(error) => respond_error(request, StatusCode(422), &error.to_string()),
            }
        }
        _ => respond_error(request, StatusCode(404), "未知矢量转换方向"),
    }
}

fn import_title(request: &Request) -> Option<String> {
    let raw = request
        .headers()
        .iter()
        .find(|candidate| candidate.field.equiv("X-UniPPT-Filename"))?
        .value
        .as_str();
    decode_import_title(raw)
}

fn export_job_id(request: &Request) -> Option<String> {
    let value = request
        .headers()
        .iter()
        .find(|candidate| candidate.field.equiv("X-UniPPT-Export-Job"))?
        .value
        .as_str()
        .trim();
    render_export::valid_job_id(value).then(|| value.to_string())
}

fn video_export_options(
    request: &Request,
    format: render_export::ExportFormat,
) -> Result<Option<render_export::VideoExportOptions>, String> {
    if !matches!(
        format,
        render_export::ExportFormat::Video | render_export::ExportFormat::VideoFast
    ) {
        return Ok(None);
    }
    let defaults = render_export::VideoExportOptions::defaults(format);
    let parse_number = |name: &str, fallback: u32| -> Result<u32, String> {
        let Some(value) = request_header(request, name) else {
            return Ok(fallback);
        };
        value
            .parse::<u32>()
            .map_err(|_| format!("invalid {name} header"))
    };
    let profile = request_header(request, "X-UniPPT-Video-Profile")
        .unwrap_or(&defaults.profile)
        .trim()
        .to_ascii_lowercase();
    let muted = match request_header(request, "X-UniPPT-Video-Muted") {
        None => defaults.muted,
        Some("1" | "true" | "yes" | "on") => true,
        Some("0" | "false" | "no" | "off") => false,
        Some(_) => return Err("invalid X-UniPPT-Video-Muted header".into()),
    };
    render_export::VideoExportOptions {
        width: parse_number("X-UniPPT-Video-Width", defaults.width)?,
        height: parse_number("X-UniPPT-Video-Height", defaults.height)?,
        fps: parse_number("X-UniPPT-Video-Fps", defaults.fps)?,
        profile,
        muted,
    }
    .validate()
    .map(Some)
}

fn request_header<'a>(request: &'a Request, name: &str) -> Option<&'a str> {
    request
        .headers()
        .iter()
        .find(|candidate| candidate.field.to_string().eq_ignore_ascii_case(name))
        .map(|candidate| candidate.value.as_str().trim())
}

fn decode_import_title(raw: &str) -> Option<String> {
    let mut decoded = Vec::with_capacity(raw.len());
    let bytes = raw.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let (Some(high), Some(low)) =
                (hex_digit(bytes[index + 1]), hex_digit(bytes[index + 2]))
            {
                decoded.push((high << 4) | low);
                index += 3;
                continue;
            }
        }
        decoded.push(bytes[index]);
        index += 1;
    }
    let decoded = String::from_utf8(decoded).ok()?;
    let basename = decoded
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(&decoded)
        .trim();
    if basename.is_empty() {
        return None;
    }
    let title = basename
        .strip_suffix(".pptx")
        .or_else(|| basename.strip_suffix(".PPTX"))
        .unwrap_or(basename)
        .trim();
    (!title.is_empty()).then(|| title.to_owned())
}

fn hex_digit(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn parse_cached_asset_path(path: &str) -> Option<(&str, &str)> {
    let rest = path.strip_prefix("/api/cache/")?;
    let (cache_id, asset_id) = rest.split_once("/asset/")?;
    let cache_digest = cache_id.strip_prefix("pptx-")?;
    if cache_digest.len() != 64
        || asset_id.len() != 64
        || !cache_digest.bytes().all(|byte| byte.is_ascii_hexdigit())
        || !asset_id.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return None;
    }
    Some((cache_id, asset_id))
}

fn serve_cached_asset(
    request: Request,
    document_cache: &SharedDocumentCache,
    cache_id: &str,
    asset_id: &str,
) -> std::io::Result<()> {
    let asset = lock_cache(document_cache)
        .get(cache_id)
        .and_then(|document| document.assets.get(asset_id).cloned());
    let Some(asset) = asset else {
        return respond_error(request, StatusCode(404), "缓存资源不存在或已过期");
    };
    respond_arc(
        request,
        StatusCode(200),
        asset.browser_bytes,
        vec![
            header("Content-Type", &asset.browser_mime_type),
            header("Cache-Control", "private, max-age=31536000, immutable"),
            header("X-Content-Type-Options", "nosniff"),
            header("ETag", &format!("\"{asset_id}\"")),
        ],
    )
}

fn static_content_hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn render_versioned_index(web_root: &Path, template: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut html = String::from_utf8(template.to_vec()).map_err(|error| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("web/index.html must be UTF-8: {error}"),
        )
    })?;
    let mut cursor = 0usize;
    while let Some(relative_start) = html[cursor..].find("{{asset:") {
        let start = cursor + relative_start;
        let path_start = start + "{{asset:".len();
        let Some(relative_end) = html[path_start..].find("}}") else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "unterminated {{asset:...}} token in web/index.html",
            ));
        };
        let end = path_start + relative_end;
        let url_path = html[path_start..end].to_string();
        let relative = url_path.trim_start_matches('/');
        if !url_path.starts_with('/')
            || relative.is_empty()
            || relative
                .split('/')
                .any(|segment| segment == ".." || segment.contains('\\'))
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid static asset token: {url_path}"),
            ));
        }
        let bytes = fs::read(web_root.join(relative)).map_err(|error| {
            std::io::Error::new(
                error.kind(),
                format!("versioned asset {url_path} is missing: {error}"),
            )
        })?;
        let digest = static_content_hash(&bytes);
        let replacement = format!("{url_path}?v={}", &digest[..16]);
        html.replace_range(start..end + 2, &replacement);
        cursor = start + replacement.len();
    }
    Ok(html.into_bytes())
}

fn query_value<'a>(url: &'a str, key: &str) -> Option<&'a str> {
    url.split_once('?')?.1.split('&').find_map(|field| {
        let (name, value) = field.split_once('=')?;
        (name == key).then_some(value)
    })
}

fn valid_cold_run(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn append_cold_run_to_versioned_urls(bytes: Vec<u8>, cold_run: &str) -> Vec<u8> {
    let Ok(source) = String::from_utf8(bytes) else {
        return Vec::new();
    };
    let mut rendered = String::with_capacity(source.len() + 256);
    let mut cursor = 0usize;
    while let Some(relative) = source[cursor..].find("?v=") {
        let version_start = cursor + relative;
        rendered.push_str(&source[cursor..version_start + 3]);
        let hash_start = version_start + 3;
        let hash_len = source[hash_start..]
            .bytes()
            .take_while(u8::is_ascii_hexdigit)
            .count();
        if hash_len < 12 {
            cursor = hash_start;
            continue;
        }
        let hash_end = hash_start + hash_len;
        rendered.push_str(&source[hash_start..hash_end]);
        rendered.push_str("&coldRun=");
        rendered.push_str(cold_run);
        cursor = hash_end;
    }
    rendered.push_str(&source[cursor..]);
    rendered.into_bytes()
}

fn request_header_value<'a>(request: &'a Request, name: &str) -> Option<&'a str> {
    request
        .headers()
        .iter()
        .find(|candidate| candidate.field.to_string().eq_ignore_ascii_case(name))
        .map(|candidate| candidate.value.as_str())
}

fn accepts_brotli(request: &Request) -> bool {
    let Some(value) = request_header_value(request, "Accept-Encoding") else {
        return false;
    };
    let mut wildcard = None;
    for item in value.split(',') {
        let mut fields = item.trim().split(';');
        let encoding = fields.next().unwrap_or_default().trim();
        let quality = fields
            .find_map(|field| field.trim().strip_prefix("q="))
            .and_then(|value| value.parse::<f32>().ok())
            .unwrap_or(1.0);
        if encoding.eq_ignore_ascii_case("br") {
            return quality > 0.0;
        }
        if encoding == "*" {
            wildcard = Some(quality > 0.0);
        }
    }
    wildcard.unwrap_or(false)
}

fn is_brotli_compressible(content_type: &str, length: usize) -> bool {
    length >= 1024
        && (content_type.starts_with("text/")
            || content_type == "image/svg+xml"
            || content_type.starts_with("application/json"))
}

fn static_permissions_policy(is_index: bool) -> Option<&'static str> {
    is_index.then_some("local-fonts=(self)")
}

fn serve_static(request: Request, static_site: &StaticSite, url_path: &str) -> std::io::Result<()> {
    let relative = if url_path == "/" {
        "index.html"
    } else {
        url_path.trim_start_matches('/')
    };
    if relative
        .split('/')
        .any(|segment| segment == ".." || segment.contains('\\'))
    {
        return respond_error(request, StatusCode(400), "非法路径");
    }
    let path = static_site.root.join(relative);
    let is_index = relative == "index.html";
    let source_bytes = if is_index {
        Ok(static_site.index_html.as_ref().to_vec())
    } else {
        fs::read(&path)
    };
    match source_bytes {
        Ok(source_bytes) => {
            let content_type = match path.extension().and_then(|ext| ext.to_str()) {
                Some("html") => "text/html; charset=utf-8",
                Some("css") => "text/css; charset=utf-8",
                Some("js") => "text/javascript; charset=utf-8",
                Some("svg") => "image/svg+xml",
                Some("png") => "image/png",
                Some("woff2") => "font/woff2",
                Some("woff") => "font/woff",
                Some("ttf") => "font/ttf",
                _ => "application/octet-stream",
            };
            let source_digest = static_content_hash(&source_bytes);
            let cold_run =
                query_value(request.url(), "coldRun").filter(|value| valid_cold_run(value));
            let bytes = match cold_run {
                Some(value) if is_index || path.extension().is_some_and(|ext| ext == "css") => {
                    append_cold_run_to_versioned_urls(source_bytes, value)
                }
                _ => source_bytes,
            };
            let entity_digest = static_content_hash(&bytes);
            let use_brotli =
                accepts_brotli(&request) && is_brotli_compressible(content_type, bytes.len());
            let encoded = use_brotli
                .then(|| static_site.brotli_bytes(&entity_digest, &bytes))
                .transpose()?;
            let etag = format!(
                "\"{}{}\"",
                entity_digest,
                if use_brotli { "-br" } else { "" }
            );
            let version_matches = query_value(request.url(), "v")
                .is_some_and(|version| version.len() >= 12 && source_digest.starts_with(version));
            let cache_control = if cold_run.is_some() {
                "no-store"
            } else if is_index {
                "no-cache"
            } else if version_matches {
                "public, max-age=31536000, immutable"
            } else {
                "no-cache"
            };
            let content_length = encoded
                .as_ref()
                .map_or(bytes.len(), |compressed| compressed.len())
                .to_string();
            if request_header_value(&request, "If-None-Match") == Some(etag.as_str()) {
                let mut response = Response::empty(StatusCode(304))
                    .with_header(header("Cache-Control", cache_control))
                    .with_header(header("ETag", &etag))
                    .with_header(header("Vary", "Accept-Encoding"))
                    .with_header(header("Server", "UniPPT"));
                if let Some(policy) = static_permissions_policy(is_index) {
                    response = response.with_header(header("Permissions-Policy", policy));
                }
                return request.respond(response);
            }
            let mut headers = vec![
                header("Content-Type", content_type),
                header("Content-Length", &content_length),
                header("Cache-Control", cache_control),
                header("ETag", &etag),
                header("Vary", "Accept-Encoding"),
                header("X-Content-Type-Options", "nosniff"),
            ];
            if use_brotli {
                headers.push(header("Content-Encoding", "br"));
            }
            if let Some(policy) = static_permissions_policy(is_index) {
                headers.push(header("Permissions-Policy", policy));
            }
            if request.method() == &Method::Head {
                headers.push(header("Server", "UniPPT"));
                request.respond(
                    Response::new(
                        StatusCode(200),
                        headers,
                        Cursor::new(Vec::<u8>::new()),
                        Some(content_length.parse().expect("static content length")),
                        None,
                    )
                    .with_chunked_threshold(usize::MAX),
                )
            } else {
                let body = encoded.unwrap_or_else(|| Arc::from(bytes));
                respond_arc(request, StatusCode(200), body, headers)
            }
        }
        Err(_) => respond_error(request, StatusCode(404), "页面不存在"),
    }
}

fn respond_json(request: Request, status: StatusCode, body: &str) -> std::io::Result<()> {
    request.respond(
        Response::from_string(body)
            .with_status_code(status)
            .with_header(header("Content-Type", "application/json; charset=utf-8"))
            .with_header(header("Cache-Control", "no-store"))
            .with_header(header("Server", "UniPPT")),
    )
}

fn respond_json_arc(
    request: Request,
    status: StatusCode,
    body: Arc<[u8]>,
    mut headers: Vec<Header>,
) -> std::io::Result<()> {
    headers.push(header("Content-Type", "application/json; charset=utf-8"));
    headers.push(header("Cache-Control", "no-store"));
    respond_arc(request, status, body, headers)
}

fn respond_pptx(request: Request, bytes: Vec<u8>) -> std::io::Result<()> {
    request.respond(
        Response::from_data(bytes)
            .with_status_code(StatusCode(200))
            .with_header(header(
                "Content-Type",
                "application/vnd.openxmlformats-officedocument.presentationml.presentation",
            ))
            .with_header(header(
                "Content-Disposition",
                "attachment; filename=unippt-export.pptx",
            ))
            .with_header(header("Cache-Control", "no-store"))
            .with_header(header("Server", "UniPPT")),
    )
}

fn respond_pptx_arc(
    request: Request,
    bytes: Arc<[u8]>,
    document: &CachedDocument,
) -> std::io::Result<()> {
    let mut headers = cache_response_headers(document, "hit");
    headers.extend([
        header(
            "Content-Type",
            "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        ),
        header(
            "Content-Disposition",
            "attachment; filename=unippt-export.pptx",
        ),
        header("Cache-Control", "no-store"),
    ]);
    respond_arc(request, StatusCode(200), bytes, headers)
}

fn respond_download(
    request: Request,
    bytes: Vec<u8>,
    content_type: &str,
    filename: &str,
) -> std::io::Result<()> {
    request.respond(
        Response::from_data(bytes)
            .with_status_code(StatusCode(200))
            .with_header(header("Content-Type", content_type))
            .with_header(header(
                "Content-Disposition",
                &format!("attachment; filename={filename}"),
            ))
            .with_header(header("Cache-Control", "no-store"))
            .with_header(header("Server", "UniPPT")),
    )
}

fn respond_download_arc(
    request: Request,
    bytes: Arc<[u8]>,
    content_type: &str,
    filename: &str,
    document: &CachedDocument,
) -> std::io::Result<()> {
    let mut headers = cache_response_headers(document, "hit");
    headers.extend([
        header("Content-Type", content_type),
        header(
            "Content-Disposition",
            &format!("attachment; filename={filename}"),
        ),
        header("Cache-Control", "no-store"),
    ]);
    respond_arc(request, StatusCode(200), bytes, headers)
}

fn respond_rendered_arc(
    request: Request,
    bytes: Arc<[u8]>,
    content_type: &str,
    filename: &str,
    document: &CachedDocument,
    renderer: &str,
) -> std::io::Result<()> {
    let mut headers = cache_response_headers(document, "hit");
    headers.extend([
        header("Content-Type", content_type),
        header(
            "Content-Disposition",
            &format!("attachment; filename={filename}"),
        ),
        header("Cache-Control", "no-store"),
        header("X-UniPPT-Export-Renderer", renderer),
    ]);
    respond_arc(request, StatusCode(200), bytes, headers)
}

fn respond_exported_artifact(
    request: Request,
    artifact: render_export::ExportedArtifact,
    document: Option<&CachedDocument>,
) -> std::io::Result<()> {
    let file = match artifact.open() {
        Ok(file) => file,
        Err(error) => return respond_error(request, StatusCode(500), &error),
    };
    let length = match artifact
        .len()
        .and_then(|length| usize::try_from(length).map_err(|_| "导出文件超过平台响应上限".into()))
    {
        Ok(length) => length,
        Err(error) => return respond_error(request, StatusCode(500), &error),
    };
    let mut headers = document
        .map(|document| cache_response_headers(document, "hit"))
        .unwrap_or_default();
    headers.extend([
        header("Content-Type", artifact.mime_type()),
        header(
            "Content-Disposition",
            &format!("attachment; filename={}", artifact.filename()),
        ),
        header("Cache-Control", "no-store"),
        header("X-UniPPT-Export-Renderer", artifact.renderer()),
        header("Server", "UniPPT"),
    ]);
    // tiny_http sends the file synchronously. `artifact` deliberately remains
    // alive until `respond` returns, then its guarded workspace is removed.
    request.respond(Response::new(
        StatusCode(200),
        headers,
        file,
        Some(length),
        None,
    ))
}

fn respond_arc(
    request: Request,
    status: StatusCode,
    bytes: Arc<[u8]>,
    mut headers: Vec<Header>,
) -> std::io::Result<()> {
    let length = bytes.len();
    headers.push(header("Server", "UniPPT"));
    request.respond(
        Response::new(status, headers, Cursor::new(bytes), Some(length), None)
            .with_chunked_threshold(usize::MAX),
    )
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "unknown panic".into()
    }
}

fn respond_error(request: Request, status: StatusCode, message: &str) -> std::io::Result<()> {
    let body = serde_json::json!({ "error": message }).to_string();
    respond_json(request, status, &body)
}

fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name.as_bytes(), value.as_bytes()).expect("valid static HTTP header")
}

fn find_web_root() -> std::io::Result<PathBuf> {
    runtime_paths::file("web/index.html")
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .map(|path| path.canonicalize().unwrap_or(path))
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "找不到 web/index.html"))
}

#[cfg(test)]
mod main_tests {
    use super::*;
    use std::io::Write;
    use std::time::Instant;
    use unippt_core::{SceneObject, ASSET_REFERENCE_PREFIX};

    fn minimal_opc_pptx() -> Vec<u8> {
        let cursor = Cursor::new(Vec::new());
        let mut writer = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        for (name, contents) in [
            ("[Content_Types].xml", b"<Types/>".as_slice()),
            ("_rels/.rels", b"<Relationships/>".as_slice()),
            ("ppt/presentation.xml", b"<p:presentation/>".as_slice()),
        ] {
            writer.start_file(name, options).unwrap();
            writer.write_all(contents).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    #[test]
    fn import_filename_is_url_decoded_and_extension_is_removed() {
        assert_eq!(
            decode_import_title("%E7%AB%8B%E4%BD%93%E8%8A%B1%E6%9C%B5.pptx"),
            Some("立体花朵".into())
        );
        assert_eq!(
            decode_import_title(r"D%3A%5Cslides%5C9.PPTX"),
            Some("9".into())
        );
        assert_eq!(decode_import_title("%20.pptx"), None);
    }

    #[test]
    fn cached_asset_routes_require_content_addressed_ids() {
        let cache_id = format!("pptx-{}", "a".repeat(64));
        let asset_id = "b".repeat(64);
        let path = format!("/api/cache/{cache_id}/asset/{asset_id}");
        assert_eq!(
            parse_cached_asset_path(&path),
            Some((cache_id.as_str(), asset_id.as_str()))
        );
        assert!(parse_cached_asset_path("/api/cache/pptx-bad/asset/nope").is_none());
        assert!(parse_cached_asset_path(&format!("{path}/extra")).is_none());
    }

    #[test]
    fn portable_import_reuses_the_verified_scene_without_native_reparse() {
        let presentation = minimal_opc_pptx();
        assert_eq!(
            portable_presentation_digest(&presentation).unwrap(),
            DocumentCache::digest(&presentation)
        );
        assert!(portable_presentation_digest(b"PK\x03\x04 fake package").is_err());
        assert!(portable_presentation_digest(b"not a ZIP").is_err());
    }

    #[test]
    fn reimported_source_rebuilds_from_the_native_opc_snapshot() {
        let source = minimal_opc_pptx();
        let digest = DocumentCache::digest(&source);
        let id = DocumentCache::id_for_digest(&digest);
        let mut deck = Deck::demo();
        deck.source_import_id = Some(id.clone());
        let mut assets = AssetCatalog::default();
        externalize_deck_assets(&mut deck, &id, &mut assets).unwrap();
        let deck_json: Arc<[u8]> = Arc::from(serde_json::to_vec(&deck).unwrap());
        let opc = Arc::new(opc_snapshot::explode(&source).unwrap());
        let expected_opc_digest = opc.digest();
        let mut cache = DocumentCache::new(2, 1024 * 1024);
        let original = cache
            .insert(
                digest,
                Arc::clone(&opc),
                Arc::new(deck.clone()),
                Arc::new(deck),
                deck_json,
                Arc::new(assets),
            )
            .unwrap();
        let mut edited = (*original.deck).clone();
        edited.title = "edited".into();
        cache
            .update_deck(
                &original.id,
                1,
                Arc::new(edited),
                4096,
                Arc::clone(&original.assets),
            )
            .unwrap();
        let reopened = cache.get_by_digest(&digest).unwrap();
        let cache = Arc::new(Mutex::new(cache));
        let exported = cached_native_pptx(&cache, &reopened).unwrap();
        assert_eq!(
            opc_snapshot::explode(&exported).unwrap().digest(),
            expected_opc_digest
        );
    }

    #[test]
    fn panic_message_reads_str_and_string_payloads() {
        assert_eq!(panic_message(Box::new("boom")), "boom");
        assert_eq!(panic_message(Box::new(String::from("owned"))), "owned");
        assert_eq!(panic_message(Box::new(1u32)), "unknown panic");
    }

    #[test]
    fn interactive_requests_are_isolated_from_long_running_work() {
        assert_eq!(request_lane(&Method::Get), RequestLane::Interactive);
        assert_eq!(request_lane(&Method::Head), RequestLane::Interactive);
        assert_eq!(request_lane(&Method::Post), RequestLane::Work);
        assert_eq!(request_lane(&Method::Put), RequestLane::Work);
        assert_eq!(request_lane(&Method::Delete), RequestLane::Work);
    }

    #[test]
    fn editor_shell_rewrites_every_static_asset_token_to_a_content_hash() {
        let root = find_web_root().expect("web root");
        let template = fs::read(root.join("index.html")).expect("index template");
        let rendered = String::from_utf8(render_versioned_index(&root, &template).unwrap())
            .expect("rendered UTF-8 index");
        assert!(!rendered.contains("{{asset:"));
        assert!(rendered.contains("/app.js?v="));
        assert!(rendered.contains("/style.css?v="));
        assert!(rendered.contains("/vendor/katex/katex.min.js?v="));
        assert!(!rendered.contains("fonts.googleapis.com"));
        assert!(!rendered.contains("cdn.jsdelivr.net"));
    }

    #[test]
    fn top_level_editor_allows_only_same_origin_local_font_access() {
        assert_eq!(static_permissions_policy(true), Some("local-fonts=(self)"));
        assert_eq!(static_permissions_policy(false), None);
    }

    #[test]
    fn static_content_hash_changes_when_the_asset_changes() {
        let first = static_content_hash(b"asset-v1");
        let second = static_content_hash(b"asset-v2");
        assert_eq!(first.len(), 64);
        assert_ne!(first, second);
    }

    #[test]
    fn brotli_static_assets_round_trip_and_shrink_editor_javascript() {
        let root = find_web_root().expect("web root");
        let site = StaticSite::load(root.clone()).expect("static site");
        let source = fs::read(root.join("app.js")).expect("app.js");
        let digest = static_content_hash(&source);
        let compressed = site.brotli_bytes(&digest, &source).expect("brotli");
        let mut decoded = Vec::new();
        brotli::BrotliDecompress(&mut Cursor::new(compressed), &mut decoded)
            .expect("brotli round trip");
        assert_eq!(decoded, source);
        assert!(decoded.len() / 2 > site.brotli_bytes(&digest, &decoded).unwrap().len());
    }

    #[test]
    fn only_text_like_static_assets_are_brotli_candidates() {
        assert!(is_brotli_compressible(
            "text/javascript; charset=utf-8",
            4096
        ));
        assert!(is_brotli_compressible("image/svg+xml", 4096));
        assert!(!is_brotli_compressible("font/woff2", 4096));
        assert!(!is_brotli_compressible("text/css", 100));
    }

    #[test]
    fn cold_run_rewrites_versioned_urls_without_touching_plain_queries() {
        let source = br#"<link href=\"/style.css?v=0123456789abcdef\"><a href=\"/?v=short\">"#;
        let rendered = String::from_utf8(append_cold_run_to_versioned_urls(
            source.to_vec(),
            "chrome-ab-01",
        ))
        .unwrap();
        assert!(rendered.contains("/style.css?v=0123456789abcdef&coldRun=chrome-ab-01"));
        assert!(rendered.contains("/?v=short"));
        assert!(valid_cold_run("chrome-ab-01"));
        assert!(!valid_cold_run("../bad"));
    }

    #[test]
    #[ignore = "large corpus performance acceptance; run explicitly"]
    fn pptx_31_compact_import_keeps_362_references_on_7_unique_assets() {
        let path = std::env::var_os("UNIPPT_31_PPTX")
        .map(PathBuf::from)
        .expect("Set UNIPPT_31_PPTX to the external corpus fixture path");
        let bytes = fs::read(&path).unwrap_or_else(|error| {
            panic!("read compact import benchmark {}: {error}", path.display())
        });
        let started = Instant::now();
        let imported = import_pptx_compact(&bytes).unwrap();
        let elapsed = started.elapsed();
        assert_eq!(imported.assets.len(), 7);
        assert_eq!(compact_asset_occurrences(&imported.deck), 362);
        let compact_json = serde_json::to_vec(&imported.deck).unwrap();
        assert!(!String::from_utf8_lossy(&compact_json).contains(";base64,"));

        let mut deck = imported.deck;
        let mut catalog = AssetCatalog::from_imported_assets(imported.assets).unwrap();
        let cache_id = DocumentCache::id_for_digest(&DocumentCache::digest(&bytes));
        externalize_deck_assets(&mut deck, &cache_id, &mut catalog).unwrap();
        let bound_json = serde_json::to_vec(&deck).unwrap();
        assert_eq!(catalog.len(), 7);
        assert!(!String::from_utf8_lossy(&bound_json).contains(ASSET_REFERENCE_PREFIX));
        eprintln!(
            "31.pptx compact server acceptance: import={}ms compact_json={} bytes assets=7 refs=362",
            elapsed.as_millis(),
            compact_json.len()
        );
    }

    fn compact_asset_occurrences(deck: &Deck) -> usize {
        let mut count = deck.fonts.len();
        for slide in &deck.slides {
            count += usize::from(slide.background_asset.is_some());
            count += object_asset_occurrences(&slide.master_objects);
            count += object_asset_occurrences(&slide.layout_objects);
            count += object_asset_occurrences(&slide.objects);
        }
        count
    }

    fn object_asset_occurrences(objects: &[SceneObject]) -> usize {
        objects
            .iter()
            .map(|object| {
                usize::from(object.asset.is_some())
                    + usize::from(object.shape_fill_asset.is_some())
                    + object.media.as_ref().map_or(0, |media| {
                        usize::from(media.asset.is_some())
                            + usize::from(media.playback_asset.is_some())
                    })
                    + object_asset_occurrences(&object.children)
            })
            .sum()
    }
}
