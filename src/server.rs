use axum::{
    extract::Query,
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Json, Response},
    routing::get,
    Router,
};
use clap::Parser;
use rust_embed::Embed;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::net::SocketAddr;
use windows_iso_downloader::*;

// ── CLI ──────────────────────────────────────────────────────────────────────

/// wisodown-server — self-hosted Windows ISO Downloader API + web UI.
#[derive(Parser, Debug)]
#[command(name = "wisodown-server", version, about)]
struct Cli {
    /// Port to listen on.
    #[arg(short, long, default_value_t = 3000)]
    port: u16,

    /// Host / bind address.
    #[arg(short = 'H', long, default_value = "0.0.0.0")]
    host: String,
}

// ── Embedded frontend assets (built from frontend/dist/client/) ──────────────

#[derive(Embed)]
#[folder = "frontend/dist"]
struct Assets;

async fn serve_static(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    match Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, mime.as_ref().to_string())],
                content.data.to_vec(),
            )
                .into_response()
        }
        None => {
            // SPA fallback
            match Assets::get("index.html") {
                Some(content) => (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, "text/html".to_string())],
                    content.data.to_vec(),
                )
                    .into_response(),
                None => StatusCode::NOT_FOUND.into_response(),
            }
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn resolve(edition: &str) -> Result<(String, String), String> {
    match resolve_edition(edition) {
        Ok((eid, purl)) => Ok((eid.to_string(), purl.to_string())),
        Err(_) => {
            // Treat as raw numeric edition ID → default to Win11 x64 page
            if edition.chars().all(|c| c.is_ascii_digit()) && !edition.is_empty() {
                Ok((edition.to_string(), PAGE_URL_X64.to_string()))
            } else {
                Err(format!(
                    "Unknown edition '{}'. Use 'x64', 'arm64', 'win10', \
                     'win11-cn-home', 'win11-cn-pro', or a numeric edition ID.",
                    edition
                ))
            }
        }
    }
}

fn err_json(msg: &str, status: StatusCode) -> Response {
    (status, Json(json!({ "error": msg }))).into_response()
}

// ── Query params ─────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct EditionParams {
    edition: Option<String>,
    cookie: Option<String>,
}

#[derive(serde::Deserialize)]
struct LinksParams {
    edition: Option<String>,
    language: Option<String>,
    cookie: Option<String>,
}

// ── API routes ───────────────────────────────────────────────────────────────

async fn api_index() -> Json<Value> {
    Json(json!({
        "name": "Windows ISO Downloader API",
        "endpoints": {
            "GET /api/resolve?edition=x64": "Resolve edition alias to editionId + pageUrl",
            "GET /api/skus?edition=x64": "List available languages for an edition",
            "GET /api/links?edition=x64&language=English": "Get download links",
            "GET /api/hashes?edition=x64": "Get SHA-256 hashes from Microsoft",
        },
        "editions": ["x64", "arm64", "win10", "win11-cn-home", "win11-cn-pro"],
    }))
}

async fn api_resolve(Query(params): Query<EditionParams>) -> Response {
    let Some(edition) = params.edition.as_deref() else {
        return err_json("Missing ?edition= parameter", StatusCode::BAD_REQUEST);
    };
    match resolve(edition) {
        Ok((edition_id, page_url)) => {
            Json(json!({ "editionId": edition_id, "pageUrl": page_url })).into_response()
        }
        Err(msg) => err_json(&msg, StatusCode::BAD_REQUEST),
    }
}

async fn api_skus(Query(params): Query<EditionParams>) -> Response {
    let Some(edition) = params.edition.as_deref() else {
        return err_json("Missing ?edition= parameter", StatusCode::BAD_REQUEST);
    };
    let (edition_id, page_url) = match resolve(edition) {
        Ok(v) => v,
        Err(msg) => return err_json(&msg, StatusCode::BAD_REQUEST),
    };

    let client = match MsDownloadClient::init(page_url, params.cookie, false).await {
        Ok(c) => c,
        Err(e) => return err_json(&e.to_string(), StatusCode::INTERNAL_SERVER_ERROR),
    };
    match client.get_skus(&edition_id).await {
        Ok(skus) => Json(serde_json::to_value(&skus).unwrap()).into_response(),
        Err(e) => err_json(&e.to_string(), StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[derive(Serialize)]
struct LinksResult {
    edition: String,
    language: String,
    #[serde(rename = "localizedLanguage")]
    localized_language: String,
    filename: Option<String>,
    #[serde(rename = "expiresAt")]
    expires_at: Option<String>,
    downloads: Vec<DownloadLink>,
    hashes: HashMap<String, String>,
}

#[derive(Serialize)]
struct DownloadLink {
    name: String,
    url: String,
}

async fn api_links(Query(params): Query<LinksParams>) -> Response {
    let Some(edition) = params.edition.as_deref() else {
        return err_json("Missing ?edition= parameter", StatusCode::BAD_REQUEST);
    };
    let Some(language) = params.language.as_deref() else {
        return err_json("Missing ?language= parameter", StatusCode::BAD_REQUEST);
    };

    let (edition_id, page_url) = match resolve(edition) {
        Ok(v) => v,
        Err(msg) => return err_json(&msg, StatusCode::BAD_REQUEST),
    };

    let client = match MsDownloadClient::init(page_url, params.cookie, false).await {
        Ok(c) => c,
        Err(e) => return err_json(&e.to_string(), StatusCode::INTERNAL_SERVER_ERROR),
    };

    let skus = match client.get_skus(&edition_id).await {
        Ok(s) => s,
        Err(e) => return err_json(&e.to_string(), StatusCode::INTERNAL_SERVER_ERROR),
    };

    let lang_lower = language.to_ascii_lowercase();
    let sku = skus
        .iter()
        .find(|s| s.language.to_ascii_lowercase() == lang_lower)
        .or_else(|| {
            skus.iter()
                .find(|s| s.localized_language.to_ascii_lowercase() == lang_lower)
        })
        .or_else(|| {
            skus.iter()
                .find(|s| s.language.to_ascii_lowercase().starts_with(&lang_lower))
        });

    let Some(sku) = sku else {
        let available: Vec<&str> = skus.iter().map(|s| s.language.as_str()).collect();
        return err_json(
            &format!(
                "Language '{}' not found. Available: {}",
                language,
                available.join(", ")
            ),
            StatusCode::BAD_REQUEST,
        );
    };

    let (resp, hashes) = tokio::join!(
        client.get_download_links(&sku.id),
        client.fetch_page_hashes()
    );

    let resp = match resp {
        Ok(r) => r,
        Err(e) => return err_json(&e.to_string(), StatusCode::INTERNAL_SERVER_ERROR),
    };

    let result = LinksResult {
        edition: edition.to_string(),
        language: sku.language.clone(),
        localized_language: sku.localized_language.clone(),
        filename: sku.friendly_file_names.first().cloned(),
        expires_at: resp.download_expiration_datetime,
        downloads: resp
            .product_download_options
            .iter()
            .map(|o| DownloadLink {
                name: o.name.clone(),
                url: o.uri.clone(),
            })
            .collect(),
        hashes,
    };

    Json(serde_json::to_value(&result).unwrap()).into_response()
}

async fn api_hashes(Query(params): Query<EditionParams>) -> Response {
    let Some(edition) = params.edition.as_deref() else {
        return err_json("Missing ?edition= parameter", StatusCode::BAD_REQUEST);
    };
    let (_, page_url) = match resolve(edition) {
        Ok(v) => v,
        Err(msg) => return err_json(&msg, StatusCode::BAD_REQUEST),
    };

    let client = match MsDownloadClient::init(page_url, params.cookie, false).await {
        Ok(c) => c,
        Err(e) => return err_json(&e.to_string(), StatusCode::INTERNAL_SERVER_ERROR),
    };

    let hashes = client.fetch_page_hashes().await;
    Json(serde_json::to_value(&hashes).unwrap()).into_response()
}

async fn docs_redirect() -> Response {
    (
        StatusCode::FOUND,
        [(header::LOCATION, "https://wisodocs.krnl64.win")],
    )
        .into_response()
}

// ── Main ─────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let api = Router::new()
        .route("/", get(api_index))
        .route("/resolve", get(api_resolve))
        .route("/skus", get(api_skus))
        .route("/links", get(api_links))
        .route("/hashes", get(api_hashes));

    let app = Router::new()
        .nest("/api", api)
        .route("/docs", get(docs_redirect))
        .route("/docs/", get(docs_redirect))
        .fallback(get(serve_static));

    let addr: SocketAddr = format!("{}:{}", cli.host, cli.port)
        .parse()
        .expect("Invalid host:port");
    eprintln!("wisodown-server listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    eprintln!("\nShutting down.");
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to listen for Ctrl+C");
}
