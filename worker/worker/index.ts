import init, {
  WasmClient,
  resolveEdition,
} from "@ntkrnl64/windows-iso-downloader";
import wasm from "@ntkrnl64/windows-iso-downloader/windows_iso_downloader_bg.wasm";

let initialized = false;

async function ensureInit() {
  if (!initialized) {
    await init(wasm);
    initialized = true;
  }
}

function json(data: unknown, status = 200) {
  return Response.json(data, {
    status,
    headers: { "Access-Control-Allow-Origin": "*" },
  });
}

function error(message: string, status = 400) {
  return json({ error: message }, status);
}

interface Sku {
  Id: string;
  Language: string;
  LocalizedLanguage: string;
  FriendlyFileNames: string[];
}

interface DownloadOption {
  Name: string;
  Uri: string;
}

interface DownloadResponse {
  ProductDownloadOptions: DownloadOption[];
  DownloadExpirationDatetime: string | null;
}

async function handleResolve(edition: string) {
  try {
    return json(resolveEdition(edition));
  } catch (e: unknown) {
    return error((e as Error).message);
  }
}

async function handleSkus(edition: string, cookie: string | null) {
  const { editionId, pageUrl } = resolveEdition(edition) as {
    editionId: string;
    pageUrl: string;
  };
  const client = await WasmClient.create(pageUrl, cookie);
  const skus = (await client.getSkus(editionId)) as Sku[];
  client.free();
  return json(skus);
}

async function handleLinks(
  edition: string,
  language: string,
  cookie: string | null,
) {
  const { editionId, pageUrl } = resolveEdition(edition) as {
    editionId: string;
    pageUrl: string;
  };
  const client = await WasmClient.create(pageUrl, cookie);
  const skus = (await client.getSkus(editionId)) as Sku[];

  const lang = language.toLowerCase();
  const sku = skus.find(
    (s) =>
      s.Language.toLowerCase() === lang ||
      s.LocalizedLanguage.toLowerCase() === lang ||
      s.Language.toLowerCase().startsWith(lang),
  );
  if (!sku) {
    client.free();
    return error(
      `Language '${language}' not found. Available: ${skus.map((s) => s.Language).join(", ")}`,
    );
  }

  const [resp, hashes] = await Promise.all([
    client.getDownloadLinks(sku.Id) as Promise<DownloadResponse>,
    client.fetchPageHashes() as Promise<Record<string, string>>,
  ]);
  client.free();

  return json({
    edition,
    language: sku.Language,
    localizedLanguage: sku.LocalizedLanguage,
    filename: sku.FriendlyFileNames?.[0] ?? null,
    expiresAt: resp.DownloadExpirationDatetime ?? null,
    downloads: resp.ProductDownloadOptions.map((o) => ({
      name: o.Name,
      url: o.Uri,
    })),
    hashes,
  });
}

async function handleHashes(edition: string, cookie: string | null) {
  const { pageUrl } = resolveEdition(edition) as { pageUrl: string };
  const client = await WasmClient.create(pageUrl, cookie);
  const hashes = await client.fetchPageHashes();
  client.free();
  return json(hashes);
}

export default {
  async fetch(request: Request) {
    const url = new URL(request.url);

    if (url.pathname === "/docs" || url.pathname === "/docs/") {
      return Response.redirect("https://wisodocs.krnl64.win", 302);
    }

    if (!url.pathname.startsWith("/api/")) {
      return new Response(null, { status: 404 });
    }

    await ensureInit();

    const path = url.pathname.replace(/^\/api/, "").replace(/\/+$/, "") || "/";
    const params = url.searchParams;
    const cookie = params.get("cookie");

    try {
      if (path === "/") {
        return json({
          name: "Windows ISO Downloader API",
          endpoints: {
            "GET /api/resolve?edition=x64":
              "Resolve edition alias to editionId + pageUrl",
            "GET /api/skus?edition=x64":
              "List available languages for an edition",
            "GET /api/links?edition=x64&language=English": "Get download links",
            "GET /api/hashes?edition=x64": "Get SHA-256 hashes from Microsoft",
          },
          editions: ["x64", "arm64", "win10", "win11-cn-home", "win11-cn-pro"],
        });
      }

      if (path === "/resolve") {
        const edition = params.get("edition");
        if (!edition) return error("Missing ?edition= parameter");
        return await handleResolve(edition);
      }

      if (path === "/skus") {
        const edition = params.get("edition");
        if (!edition) return error("Missing ?edition= parameter");
        return await handleSkus(edition, cookie);
      }

      if (path === "/links") {
        const edition = params.get("edition");
        const language = params.get("language");
        if (!edition) return error("Missing ?edition= parameter");
        if (!language) return error("Missing ?language= parameter");
        return await handleLinks(edition, language, cookie);
      }

      if (path === "/hashes") {
        const edition = params.get("edition");
        if (!edition) return error("Missing ?edition= parameter");
        return await handleHashes(edition, cookie);
      }

      return error("Not found", 404);
    } catch (e: unknown) {
      return error((e as Error).message || String(e), 500);
    }
  },
} satisfies ExportedHandler;
