# Referencia técnica

Este documento describe el funcionamiento interno de **wisodown**: su
arquitectura, la organización de módulos, el flujo contra la API de Microsoft
que reproduce, los requisitos de compilación, la portabilidad a Linux y los
scripts auxiliares. Es el complemento de
[Cómo funciona](/guide/how-it-works), centrado en el protocolo en tiempo de
ejecución.

> Hay una versión en inglés en [Technical Reference](/guide/technical).

---

## 1. Visión general

`wisodown` es un proyecto Rust con tres modos de uso sobre una única librería:

| Modo | Target | Punto de entrada |
| ---- | ------ | ---------------- |
| CLI | nativo | `src/main.rs` → `wisodown` |
| API autoalojada + web | nativo (feature `server`) | `src/server.rs` → `wisodown-server` |
| Navegador / WASM | `wasm32` | `src/wasm.rs` → `windows_iso_downloader` (cdylib) |

Los tres comparten la misma implementación `MsDownloadClient`, de modo que el
bypass de Sentinel, la consulta de SKUs, la generación de enlaces y el scraping
de hashes viven en un solo sitio.

```
src/
├── lib.rs        # superficie pública + gating de features
├── main.rs       # CLI (clap): asistente, flags, progreso, hashing
├── server.rs     # API HTTP con axum + frontend embebido + caché SQLite
├── wasm.rs       # envoltorio wasm-bindgen
├── client.rs     # MsDownloadClient: sesión, fingerprints, llamadas API
├── download.rs   # descarga en streaming y paralela (Range), SHA-256
├── edition.rs    # alias de edición → ProductEditionId + URL de página
├── parse.rs      # scraping de HTML/JS: config, bundle SDS, tabla de hashes
└── types.rs      # modelos serde + configuración compartida
```

---

## 2. Estructura del crate

`Cargo.toml` define:

- **Librería** `windows_iso_downloader` (`src/lib.rs`), `crate-type = ["lib", "cdylib"]`.
- **Binario** `wisodown` (`src/main.rs`) — la CLI.
- **Binario** `wisodown-server` (`src/server.rs`), tras `required-features = ["server"]`.

Las dependencias se separan por target para que el mismo crate compile para
nativo y para `wasm32`:

- **Núcleo (todos los targets):** `reqwest` (solo json), `serde`, `serde_json`,
  `uuid`, `anyhow`, `sha2`, `hex`.
- **Solo nativo:** `tokio`, `clap`, `dialoguer`, `console`, `indicatif`,
  `futures-util`, `bytes`, `axum`, `tower-http`, `rust-embed`, `mime_guess`,
  `rusqlite` (bundled), `time`.
- **Solo WASM:** `js-sys`, `wasm-bindgen`, `wasm-bindgen-futures`,
  `serde-wasm-bindgen`.

### Backend TLS

`reqwest` se configura con `default-features = false` y la feature `rustls-tls`.
Esto es intencionado:

- Elimina la dependencia del OpenSSL del sistema (`native-tls` → `openssl-sys`).
- Las raíces de certificados van incluidas (`webpki-roots`), así que el binario
  se comporta igual en distros sin un almacén de CAs del sistema utilizable.

Como resultado, en Linux **no hace falta `libssl-dev` ni OpenSSL**. La única
dependencia nativa de compilación que queda es un compilador de C, necesario para
`rusqlite` con la feature `bundled` (usada solo por `wisodown-server`).

---

## 3. Referencia de módulos

### `client.rs` — `MsDownloadClient`

| Método | Función |
| ------ | ------- |
| `init(page_url, cookie, debug)` | Crea el cliente HTTP, carga la página, extrae la config dinámica y ejecuta ambos flujos de fingerprint. |
| `get_skus(edition_id)` | Llama a `getSkuInformationByProductEdition`, devuelve `Vec<Sku>`. |
| `get_download_links(sku_id)` | Llama a `GetProductDownloadLinksBySku`, devuelve `DownloadResponse`. |
| `fetch_page_hashes()` | Raspa la tabla de SHA-256 de la página. |
| `download_file(url, path, threads, hash)` | Solo nativo; delega en `download.rs::download_url`. |

El cliente usa un `User-Agent` fijo de Chrome sobre Windows para la petición de
la *página*. Es intencionado — Microsoft redirige los UA de Windows a la Media
Creation Tool — y no tiene relación con el SO anfitrión.

### `download.rs`

Una sola conexión descarga a un fichero `.part` y calcula el hash al vuelo. Con
`--threads > 1` y un servidor que anuncie `Accept-Ranges: bytes`, el fichero se
preasigna con `set_len`, se divide en N rangos de bytes iguales y cada rango se
descarga en su propia tarea de tokio con su propio descriptor de fichero
(`seek` + `BufWriter`). El hash se calcula después en una segunda pasada
secuencial. `tokio::select!` compite la descarga contra `ctrl_c()` para cancelar
limpiamente.

### `edition.rs`

Convierte los alias de usuario (`x64`, `arm64`, `win10`, `win11-cn-home`,
`win11-cn-pro`, …) en un par `(ProductEditionId, page_url)`. También aloja
`find_sku_by_language` y `filename_from_url`.

Los alias se resuelven a los productos actuales de Microsoft (verificado en vivo
contra la API de Microsoft):

| Alias | `ProductEditionId` | Página | Versión | Arquitectura | Nombre del ISO |
| ----- | ------------------ | ------ | ------- | ------------ | -------------- |
| `x64` | `3321` | `software-download/windows11` | Windows 11 **25H2** | x64 | `Win11_25H2_<Idioma>_x64_v2.iso` |
| `arm64` | `3324` | `software-download/windows11arm64` | Windows 11 **25H2** | Arm64 | `Win11_25H2_<Idioma>_Arm64_v2.iso` |
| `win10` | `2618` | `software-download/windows10ISO` | Windows 10 **22H2** | x86 + x64 | `Win10_22H2_<Idioma>_x32v1.iso` / `…_x64v1.iso` |
| `win11-cn-home` | `3322` | `software-download/windows11` | Windows 11 **25H2** Home China | x64 | solo chino simplificado |
| `win11-cn-pro` | `3323` | `software-download/windows11` | Windows 11 **25H2** Pro China | x64 | solo chino simplificado |

Microsoft solo sirve la **última** actualización de características de cada
producto: Windows 11 **25H2** y Windows 10 **22H2**. Las ISOs de consumidor son
**multiedición** (Home, Pro, Education, …) — la clave de producto elige la
edición al instalar. No hay medios de Windows 11 de 32 bits ni medios LTSC/IoT en
las páginas públicas.

`win10` es el único alias que devuelve **dos** opciones de descarga (32 bits `x32`
y 64 bits `x64`); la CLI muestra un prompt `Select download` cuando hay más de
una opción (`x64`/`arm64` devuelven una sola). Hay 38 idiomas disponibles para
Windows 10 y para Windows 11 x64/Arm64.

### `legacy.rs`

Conecta las ediciones **Windows 7 / 8 / 8.1** con **archive.org**, ya que Microsoft
dejó de publicar esas ISOs de consumidor (su página `windows7ISO` solo muestra un
aviso de fin de soporte, y `GetProductDownloadLinksBySku` solo conoce productos
actuales).

- Una tabla curada (`LEGACY_EDITIONS`) asocia los alias a identificadores de
  archive.org: `win7` → `win7-ult-sp1-english`, `win8` → `Win8_RTM_MSDN_English`,
  `win8.1` → `win-8.1`.
- `collect_isos()` consulta `https://archive.org/metadata/<item>` en tiempo de
  ejecución y devuelve cada fichero `.iso` con su tamaño y MD5 (así la lista se
  mantiene sincronizada con archive.org).
- `LegacyIso::url()` construye `https://archive.org/download/<item>/<file>` (con
  codificación porcentual de los segmentos).
- `language_from_filename()` / `arch_from_filename()` derivan una etiqueta legible
  del nombre (`Win8.1_<Idioma>_x{32,64}.iso` o el prefijo ISO-639 en
  `en_windows_8_x64_…iso`); `filter_by_language()` hace coincidencia exacta y,
  si no, por subcadena. El código admite el código de idioma español (`es`).
- Las descargas reutilizan `download.rs` (`download_url`) con la misma maquinaria
  multihilo por rangos y luego verifican el **MD5** de archive.org (crate `md-5`).

La CLI (`main.rs::run_legacy`) gestiona `win7`/`win8`/`win8.1` antes del flujo de
Microsoft; el servidor (`server.rs`) los expone por los mismos endpoints
`/api/resolve`, `/api/skus`, `/api/links` y `/api/hashes`.

### `parse.rs`

Parseo de cadenas puro, sin dependencias de HTML:

- `extract_hidden_input` — lee `<input id="…" value="…">` (p. ej. `endpoint-svc`).
- `extract_sds_js_url` — localiza el `<script src>` del bundle SDS en la página.
- `extract_from_sds_js` — extrae `profile`, `instanceId`, `orgId` del bundle.
- `parse_page_hashes` / `lookup_page_hash` — construyen y consultan la tabla de
  SHA-256, tolerando que el campo `FriendlyFileNames` no exista.

### `types.rs`

Modelos serde. Destaca `Sku::friendly_file_names` con `#[serde(default)]` porque
la API actual de Microsoft ya no devuelve `FriendlyFileNames`; el código recurre
a `filename_from_url` cuando falta.

### `server.rs` (feature `server`)

Router `axum` que expone `/api/resolve`, `/api/skus`, `/api/links` y
`/api/hashes`, sirviendo el frontend embebido (`rust-embed`, carpeta
`frontend/dist`) con fallback SPA. Las respuestas de `/api/links` se cachean en
SQLite (`rusqlite`) con clave `(edition_id, language_key)` y un TTL derivado del
timestamp de expiración de Microsoft (o `--cache-ttl`).

Las ediciones `win7`/`win8`/`win8.1` las gestiona `legacy.rs`: `/api/resolve`
devuelve un id `legacy:<edición>` y la URL de detalles de archive.org, `/api/skus`
agrupa las ISOs en SKUs por idioma, `/api/links` devuelve las descargas de
archive.org (con los MD5 en `hashes`) y `/api/hashes` devuelve nombre → MD5. Estas
rutas no se cachean.

---

## 4. Flujo contra la API de Microsoft

```
init()
  ├─ GET page_url                         → cookies httpOnly de sesión
  ├─ parse endpoint-svc + bundle SDS      → api_base, profile, instanceId, orgId
  ├─ GET vlscppe.microsoft.com/fp/tags.js → cookies ThreatMetrix
  ├─ GET vlscppe.microsoft.com/tags       → cookies ThreatMetrix
  ├─ GET ov-df.microsoft.com/mdt.js       → `ticks` por sesión
  └─ GET ov-df.microsoft.com/?…           → cookies fptctx2 + MUID
get_skus()
  └─ GET <api_base>/getskuinformationbyproductedition
get_download_links()
  └─ GET <api_base>/GetProductDownloadLinksBySku → URLs CDN con caducidad
fetch_page_hashes()
  └─ GET page_url → parseo de la tabla <td> de hashes
download_file()
  ├─ HEAD url                             → Content-Length + Accept-Ranges
  └─ GET url (+ cabeceras Range)          → fichero .part → rename → SHA-256
```

Sentinel protege los endpoints de la API; `fptctx2` y `MUID` deben estar
presentes o la petición se rechaza con `ErrorSettings.SentinelReject`. Consulta
[Cómo funciona](/guide/how-it-works) para los detalles completos del protocolo.

---

## 5. Requisitos de compilación

| Requisito | Necesario para | Notas |
| --------- | -------------- | ----- |
| Rust stable 1.75+ | todo | se recomienda `rustup` |
| Compilador de C (`cc`/`gcc`/`clang`) | `wisodown-server` | SQLite bundled vía `rusqlite` |
| OpenSSL | — | **no necesario** (rustls) |

```sh
# Solo CLI
cargo build --release

# CLI + servidor
cargo build --release --features server
```

### Compilación cruzada a WASM

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
wasm-pack build --target web
```

---

## 6. Notas de portabilidad a Linux

El código ya es multiplataforma. En concreto:

- **Nada específico del SO.** No hay `#[cfg(windows)]`, ni `winapi`/
  `windows-sys`, ni `std::os::windows`, ni `cmd.exe`, ni rutas con `\`. Los
  caminos usan `std::path::Path` / `PathBuf` en todas partes.
- **TLS autocontenido.** `rustls` + `webpki-roots` evita la dependencia del
  OpenSSL del sistema (ver §2).
- **Señales.** La cancelación usa `tokio::signal::ctrl_c()`, que funciona en
  Unix y Windows.
- **Interfaz de terminal.** `dialoguer`/`console`/`indicatif` son
  multiplataforma.
- **Scripts.** `scripts/get-iso.sh` es Bash y ya es nativo en Linux/macOS;
  `scripts/get-iso.ps1` está escrito para ejecutarse con PowerShell 7 (`pwsh`)
  también en Linux y macOS.
- **CI.** `.github/workflows/ci.yml` compila en Ubuntu, macOS y Windows para
  evitar regresiones.

Compilación verificada en Linux x86-64 (genera un
`ELF 64-bit LSB pie executable` nativo).

---

## 7. Scripts auxiliares

### `scripts/get-iso.sh` (Linux / macOS)

Script Bash que habla con un `wisodown-server`. Requiere `curl` y `jq`. Por
defecto usa `http://localhost:3000`; cámbialo con `-a <url>` o
`WISODOWN_API=<url>`.

```sh
./scripts/get-iso.sh -e x64 -l English          # imprime el enlace
./scripts/get-iso.sh -e arm64 -l Japanese -d    # descarga
```

### `scripts/get-iso.ps1` (Windows / Linux / macOS)

Script de PowerShell 7. En Linux/macOS:

```sh
chmod +x scripts/get-iso.ps1
./scripts/get-iso.ps1 -Edition x64 -Language English
# o
pwsh ./scripts/get-iso.ps1 -Edition x64 -Language English -Download
```

Los nombres de idioma se codifican con `[uri]::EscapeDataString`, las entradas
numéricas se validan con `[int]::TryParse` y las respuestas se fuerzan a arrays
para que los casos de un solo elemento se comporten igual en todas las
plataformas.

---

## 8. Servidor autoalojado

```sh
cargo run --release --features server -- --port 3000 --host 0.0.0.0
```

| Ruta | Descripción |
| ---- | ----------- |
| `GET /api/resolve` | `?edition=` → `{ editionId, pageUrl }` |
| `GET /api/skus` | `?edition=` → lista de idiomas |
| `GET /api/links` | `?edition=&language=` → URLs de descarga + hashes (caché SQLite) |
| `GET /api/hashes` | `?edition=` → tabla SHA-256 |
| `GET /*` | SPA embebida (`frontend/dist`) |

CORS se configura con el flag repetible `--cors-origin` (`*` permite todo). El
apagado ordenado se dispara con Ctrl+C.

---

## 9. CI y releases

- **CI** (`ci.yml`): en cada push a `main` y en pull requests ejecuta
  `cargo check`/`build` en Ubuntu, macOS y Windows, con y sin la feature
  `server`.
- **Release** (`release.yml`): una matriz en Ubuntu / macOS / Windows (x86_64 e
  i686) compila y publica archivos por plataforma —
  `wisodown-linux-x86_64.zip`, `wisodown-macos-arm64.zip`,
  `wisodown-windows-x86_64.zip` y `wisodown-windows-i686.zip`. Cada archivo
  incluye ambos binarios, los READMEs EN/ES, las licencias, los scripts
  auxiliares y las guías técnicas. `workflow_dispatch` acepta un input opcional
  `tag` para (re)adjuntar los assets a una release existente.

---

## 10. Solución de problemas

| Síntoma | Causa / solución |
| ------- | ---------------- |
| `missing field 'FriendlyFileNames'` | Corregido con `#[serde(default)]`; usa `v0.1.3` o superior. |
| `ErrorSettings.SentinelReject` | Demasiadas peticiones desde tu IP. Espera, usa una VPN o pasa `--cookie`. |
| Falla la compilación por falta de `openssl` | Estás en un build antiguo con `native-tls`; actualiza para usar `rustls-tls`. |
| `cc: command not found` al compilar el servidor | Instala un compilador de C (lo necesita SQLite bundled). |
| No se devuelve enlace de descarga | SKU inválido/caducado; reintenta y/o pasa `--cookie`. |
