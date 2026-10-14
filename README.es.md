# Descargador de ISO de Windows

[English](README.md) · **Español**

Descarga ISOs de Windows 10 y Windows 11 directamente desde los servidores de
Microsoft — y también Windows 7, 8 y 8.1 desde archive.org — sin Media Creation
Tool y sin navegador.

## Características

- Descarga las ISOs directamente desde el CDN oficial de Microsoft
- Compatible con **Windows 10** (multiedición: Home/Pro, 32 y 64 bits)
- Compatible con **Windows 11 x64** y **Windows 11 arm64**
- Compatible con **Windows 7, 8 y 8.1** desde **[archive.org](https://archive.org)**
  (Microsoft ya no publica esas ISOs de consumo)
- **Soporte de ID de edición en crudo** — pasa cualquier `ProductEditionId`
  numérico de la API de Microsoft
- **Verificación SHA-256** — el hash se calcula durante la descarga y se muestra
  al terminar; usa `--verify` para fallar si no coincide
- Asistente interactivo si se ejecuta sin argumentos
- Todos los idiomas disponibles soportados
- Barra de progreso con velocidad y tiempo estimado
- Modo `--url-only` para imprimir solo el enlace de descarga

## Qué puedes descargar

| `--edition` | Versión de Windows | Arquitectura | Nombre del ISO |
| ----------- | ------------------ | ------------ | -------------- |
| `x64` | Windows 11 **25H2** | x64 (64 bits) | `Win11_25H2_<Idioma>_x64_v2.iso` |
| `arm64` | Windows 11 **25H2** | Arm64 | `Win11_25H2_<Idioma>_Arm64_v2.iso` |
| `win10` | Windows 10 **22H2** | x86 (32 bits) **y** x64 (64 bits) | `Win10_22H2_<Idioma>_x32v1.iso` / `Win10_22H2_<Idioma>_x64v1.iso` |
| `win11-cn-home` | Windows 11 **25H2** Home (China) | — | solo chino simplificado |
| `win11-cn-pro` | Windows 11 **25H2** Pro (China) | — | solo chino simplificado |
| `win7` | Windows 7 **Ultimate SP1** — archive.org | x86 (32 bits) **y** x64 (64 bits) | `Win7_Ult_SP1_English_x32.iso` / `Win7_Ult_SP1_Spanish_x32.iso` / `..._x64.iso` / `..._x64.es.iso` |
| `win8` | Windows 8 **RTM** — archive.org | x86 (32 bits) **y** x64 (64 bits) | `en_windows_8_x86_dvd_915417.iso` / `..._x64_...iso` |
| `win8.1` | Windows 8.1 **Home/Pro** — archive.org | x86 (32 bits) **y** x64 (64 bits) | `Win8.1_<Idioma>_x32.iso` / `Win8.1_<Idioma>_x64.iso` |

- Las ISOs son **multiedición** (Home/Pro/Edu/…): la clave de producto elige la edición durante la instalación.
- Hay **38 idiomas** para Windows 10/11 (x64 y Arm64).
- Windows 11 **no tiene ISO de 32 bits**; Windows 10 te deja elegir **32 o 64 bits**.
- Las imágenes **LTSC / IoT** (Enterprise) *no* están en las páginas públicas de Microsoft.
- Cualquier otro producto: pasa su `ProductEditionId` numérico con `--edition-id`.
- **Windows 7 / 8 / 8.1 se sirven desde [archive.org](https://archive.org)**, no
  desde Microsoft: esas ISOs de consumo se retiraron de las páginas públicas.
  `win8.1` es **multiidioma**; `win7`/`win8` admiten **inglés y español**
  (las ISOs en español están disponibles via archive.org). La lista de archivos
  se lee de la API de metadatos de archive.org en tiempo de ejecución y
  las descargas se verifican con el **MD5** publicado ahí (`--verify` sigue
  aceptando un SHA-256).

## Requisitos

- Un entorno de Rust (`cargo`, estable 1.75+)
- Un compilador de C (`cc`/`gcc`/`clang`) — necesario para el SQLite embebido que
  usa el servidor opcional
- **No hace falta OpenSSL del sistema**: el TLS lo aporta `rustls` con raíces
  incluidas, así que la compilación es autocontenida en cualquier SO

## Ciclo de vida del soporte de Windows

La política de soporte de Microsoft afecta a qué versiones de ISO puedes usar con seguridad con esta herramienta:

- **Windows 7** el soporte principal terminó el **13 de enero de 2015**, el soporte extendido terminó el **14 de enero de 2020**. Ya no recibe actualizaciones de seguridad.
- **Windows 8** el soporte terminó el **12 de enero de 2016**. Ya no recibe actualizaciones de seguridad.
- **Windows 8.1** el soporte terminó el **10 de enero de 2018**. Ya no recibe actualizaciones de seguridad.
- **Windows 10** el soporte principal terminó el **13 de octubre de 2020**. El soporte extendido termina el **14 de octubre de 2025**.
- **Windows 10 ESU** (Actualizaciones de seguridad extendidas) está disponible para Windows 10 Enterprise hasta **enero de 2028** (a través de Volumen Licencias). ESU recibe actualizaciones de seguridad críticas incluso después de que termina el soporte extendido.
- **Rotación de certificados raíz**: A partir de 2023, Microsoft comenzó a rotar los certificados raíz de confianza. Los sistemas que ejecutan Windows 7/8/8.1 **sin ESU** pueden perder la capacidad de conectarse a Windows Update y algunos servicios de Microsoft a medida que expiran las raíces antiguas y no se instalan las nuevas. Windows 10 con ESU recibe las actualizaciones de certificados. Esto puede afectar la capacidad de la herramienta para adquirir cookies de la API de Microsoft en sistemas sin actualizar.

> **Recomendación**: Si aún se requiere para flujos de trabajo heredados, use Windows 10 con ESU, o migre a un SO actualmente compatible. La herramienta misma se ejecuta en cualquier SO con Rust, pero los ISOs de Windows que descarga tienen el ciclo de vida anterior.

## Instalación

```sh
cargo install --path .
# binario: wisodown
```

O compila manualmente:

```sh
cargo build --release
# binario en target/release/wisodown
```

### Compilación en Linux

El proyecto es totalmente multiplataforma y compila de forma nativa en Linux —
no hay código específico de Windows (`winapi`, `#[cfg(windows)]`, rutas con
`\`, …). Solo los *productos* ISO son Windows; la herramienta funciona en
cualquier sistema donde corra Rust.

```sh
git clone https://github.com/BENDER35/wisodown-lin
cd wisodown-lin
cargo build --release
# binario: target/release/wisodown
```

Para compilar también el servidor autoalojado opcional (necesita un compilador
de C para el SQLite embebido):

```sh
cargo build --release --features server
# binario: target/release/wisodown-server
```

### Compilación en Windows

La herramienta también compila de forma nativa en Windows:

1. Instala Rust desde <https://rustup.rs> (`rustup-init.exe`). El instalador
   descarga por defecto el toolchain MSVC (necesita los *Build Tools* de
   Visual Studio C++).
2. Compila:

```powershell
git clone https://github.com/BENDER35/wisodown-lin
cd wisodown-lin
cargo build --release
# binario: target\release\wisodown.exe

# con el servidor opcional (necesita el compilador de C para el SQLite embebido):
cargo build --release --features server
# binario: target\release\wisodown-server.exe

# compilación de 32 bits (x86) — requiere el toolset C++ x86 de Visual Studio:
rustup target add i686-pc-windows-msvc
cargo build --release --target i686-pc-windows-msvc
# binario: target\i686-pc-windows-msvc\release\wisodown.exe
```

Soporte de **SO anfitrión**: **Windows 10 y Windows 11** — x64, ARM64 y x86 de
32 bits (`i686`). Windows 7/8.1 ya no están soportados por los toolchains
actuales de Rust. (Esto se refiere a *dónde se ejecuta el binario* — las ISOs que
descarga son de Windows 7/8/8.1/10/11; ver
[Qué puedes descargar](#qué-puedes-descargar).)

> **Releases precompiladas:** cada release incluye archivos listos para ejecutar:
> `wisodown-linux-x86_64.zip`, `wisodown-macos-*.zip`,
> `wisodown-windows-x86_64.zip` (64 bits) y `wisodown-windows-i686.zip`
> (32 bits). También puedes compilar desde el código fuente, o usar el script
> auxiliar `scripts/get-iso.ps1` (no requiere compilar nada).

### Scripts auxiliares

| Script | Plataforma | Requisitos |
| ------ | ---------- | ---------- |
| `scripts/get-iso.sh` | Linux / macOS | `bash`, `curl`, `jq` |
| `scripts/get-iso.ps1` | Windows / Linux / macOS (PowerShell 7+) | `pwsh` |

Ambos se comunican con un `wisodown-server` en ejecución y pueden imprimir un
enlace de descarga o descargar la ISO directamente. Por defecto usan
`http://localhost:3000` (puedes cambiarlo con `-a <url>` / `WISODOWN_API=<url>` en
bash, o `-ApiBase <url>` / `WISODOWN_API=<url>` en PowerShell), así que levanta
primero un servidor:

```sh
cargo run --release --features server -- --port 3000     # terminal 1
./scripts/get-iso.sh -e x64 -l English                   # terminal 2
```

En Linux el script de PowerShell se ejecuta con
[`pwsh`](https://learn.microsoft.com/powershell/scripting/install/installing-powershell-on-linux):

```sh
pwsh ./scripts/get-iso.ps1 -Edition x64 -Language English
# o, tras chmod +x:
./scripts/get-iso.ps1 -Edition x64 -Language English
```

## Uso

### Asistente interactivo

```sh
wisodown
```

Pregunta por la versión de Windows, el idioma y (para Windows 10) 32 o 64 bits.

### Ediciones con nombre

```sh
# Windows 11 x64, inglés
wisodown --edition x64 --language English

# Windows 11 arm64, francés
wisodown --edition arm64 --language French

# Windows 10, inglés (pregunta 32 vs 64 bits)
wisodown --edition win10 --language English

# Guardar en un directorio concreto
wisodown --edition x64 --language English --output ~/Downloads
```

### Ediciones antiguas — Windows 7 / 8 / 8.1 (archive.org)

Microsoft ya no publica las ISOs de consumo de Windows 7 / 8 / 8.1, así que estas
ediciones se descargan desde **[archive.org](https://archive.org)**:

```sh
# Windows 7 Ultimate SP1, 64 bits (español)
wisodown --edition win7 --language Spanish

# Windows 7 Ultimate SP1, 64 bits (inglés)
wisodown --edition win7 --language English

# Windows 8 RTM, 32 bits (inglés)
wisodown --edition win8 --language English

# Windows 8 RTM, 64 bits (inglés)
wisodown --edition win8 --language English

# Windows 8.1 — elige idioma; hay 32 y 64 bits
wisodown --edition win8.1 --language Spanish

# Ver todas las ISOs disponibles de una edición
wisodown --edition win8.1 --list-languages

# Solo imprimir la URL de archive.org
wisodown --edition win7 --language English --url-only
```

Para `--language`, `win8.1` usa el idioma que aparece en el nombre del archivo
(`Spanish`, `German`, `Chinese(Simplified)`, …); si hay coincidencia exacta se usa
esa, y si no, se busca por subcadena. `win7` y `win8` admiten **inglés y español**
(las ISOs en español están disponibles via archive.org).

> Son **espejos de la comunidad** alojados en Internet Archive, no servidores de
> Microsoft. Las descargas se verifican con el MD5 publicado por archive.org.

### IDs de edición en crudo

Pasa cualquier `ProductEditionId` numérico directamente, saltándote los alias:

```sh
wisodown --edition-id 2618 --language English
wisodown --edition-id 3321 --language Japanese
```

Si usas un ID de edición para una página de producto distinta de Windows 11 x64,
indica también su URL para obtener las cookies correctas:

```sh
wisodown --edition-id 2618 \
         --page-url https://www.microsoft.com/en-us/software-download/windows10ISO \
         --language English
```

### Alias de `--edition`

| Valor   | Descripción                                                     |
| ------- | --------------------------------------------------------------- |
| `x64`   | Windows 11 25H2, 64 bits (x86-64)                               |
| `arm64` | Windows 11 25H2, Arm64                                          |
| `win10` | Windows 10 22H2, multiedición (pregunta 32 vs 64 bits)          |
| `win11-cn-home` | Windows 11 25H2 Home (China, solo chino simplificado)    |
| `win11-cn-pro`  | Windows 11 25H2 Pro (China, solo chino simplificado)     |
| `win7`          | Windows 7 Ultimate SP1, inglés y español (**archive.org**)         |
| `win8`          | Windows 8 RTM, inglés (**archive.org**)                  |
| `win8.1`        | Windows 8.1 Home/Pro, multiidioma (**archive.org**)      |

También se aceptan los alias `win11`, `win11-x64`, `win11-arm64`, `amd64`,
`aarch64`, `cn-home` y `cn-pro`. Las ediciones antiguas aceptan además `windows7`,
`windows8`, `windows8.1`, `win81`, `windows81` y los valores sueltos `7`, `8`, `8.1`.

### Verificación de hash

El digest SHA-256 siempre se calcula durante la descarga y se imprime en stderr:

```text
✔ Saved to Win11_English_x64.iso
  SHA-256: b5bb9d8014a0f9b1d61e21e796d78dccdf1352f23cd32812f4850b878ae4944c
```

Usa `--verify` para exigir un hash esperado — el proceso termina con error si no
coincide:

```sh
wisodown --edition x64 --language English \
         --verify b5bb9d8014a0f9b1d61e21e796d78dccdf1352f23cd32812f4850b878ae4944c
```

### Otros flags

```sh
# Listar los idiomas disponibles para una edición
wisodown --edition win10 --list-languages

# Imprimir la URL de descarga sin descargar
wisodown --edition x64 --language English --url-only
```

### Cookies anti-bot (si fallan las descargas)

La API de Microsoft usa una capa anti-bot llamada Sentinel. La herramienta intenta
obtener las cookies necesarias automáticamente, pero si falla puedes aportarlas
a mano:

1. Abre la página de descarga en un navegador
2. Abre DevTools → Network → cualquier petición a `microsoft.com` → Request
   Headers → Cookie
3. Copia la cadena de cookies y pásala:

```sh
wisodown --edition x64 --language English --cookie "MUID=...; fptctx2=..."
```

### Modo depuración

```sh
wisodown --edition x64 --language English --debug
```

Imprime todas las peticiones y respuestas de la API, y los pasos de intercambio
de cookies, en stderr.

## Cómo funciona

1. Carga la página de descarga de Microsoft para obtener las cookies de sesión
2. Llama a `ov-df.microsoft.com` para simular el paso de fingerprinting del
   navegador (cookies fptctx2/MUID) que exige Sentinel
3. Llama a `getSkuInformationByProductEdition` para listar los SKUs de idioma
4. Llama a `GetProductDownloadLinksBySku` para obtener un enlace de CDN temporal
5. Descarga la ISO a disco en streaming, calculando SHA-256 al vuelo, con barra
   de progreso

Para las ediciones antiguas **Windows 7 / 8 / 8.1** no hay llamada a la API de
Microsoft: la herramienta consulta la **API de metadatos de archive.org** para
obtener la lista de ISOs (nombres, tamaños, MD5) y descarga desde
`https://archive.org/download/<item>/<file>`, reutilizando el mismo descargador
multihilo y comprobando el MD5 de archive.org. Admita idioma inglés y español.

## Documentación

- [Guía técnica (inglés)](docs/guide/technical.md)
- [Referencia técnica (español)](docs/guide/technical.es.md)

## Licencia

GNU General Public License 3.0
