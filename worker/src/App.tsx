import { useState } from "react";
import {
  Body1,
  Button,
  Card,
  CardHeader,
  Dialog,
  DialogBody,
  DialogContent,
  DialogSurface,
  DialogTitle,
  DialogTrigger,
  Dropdown,
  Field,
  Input,
  Link,
  makeStyles,
  MessageBar,
  MessageBarBody,
  Option,
  Spinner,
  Subtitle1,
  Text,
  Title2,
  tokens,
} from "@fluentui/react-components";
import {
  ArrowDownloadRegular,
  CheckmarkCircleRegular,
  DismissRegular,
  ShieldCheckmarkRegular,
} from "@fluentui/react-icons";

const EDITIONS = [
  { key: "x64", label: "Windows 11 (x64)" },
  { key: "arm64", label: "Windows 11 (ARM64)" },
  { key: "win10", label: "Windows 10" },
  { key: "win11-cn-home", label: "Windows 11 Home China" },
  { key: "win11-cn-pro", label: "Windows 11 Pro China" },
  { key: "custom", label: "Custom Edition ID..." },
];

interface Sku {
  Id: string;
  Language: string;
  LocalizedLanguage: string;
  FriendlyFileNames: string[];
}

interface DownloadLink {
  name: string;
  url: string;
}

interface LinksResult {
  edition: string;
  language: string;
  localizedLanguage: string;
  filename: string | null;
  expiresAt: string | null;
  downloads: DownloadLink[];
  hashes: Record<string, string>;
}

const useStyles = makeStyles({
  root: {
    display: "flex",
    flexDirection: "column",
    alignItems: "center",
    minHeight: "100vh",
    padding: "40px 16px",
  },
  container: {
    display: "flex",
    flexDirection: "column",
    gap: "20px",
    width: "100%",
    maxWidth: "560px",
  },
  header: {
    textAlign: "center",
    marginBottom: "8px",
  },
  card: {
    padding: "20px",
  },
  form: {
    display: "flex",
    flexDirection: "column",
    gap: "16px",
  },
  downloadCard: {
    padding: "16px",
    display: "flex",
    flexDirection: "column",
    gap: "12px",
  },
  downloadItem: {
    display: "flex",
    flexDirection: "column",
    gap: "8px",
  },
  meta: {
    display: "flex",
    flexDirection: "column",
    gap: "4px",
    marginTop: "4px",
  },
  hashTable: {
    width: "100%",
    borderCollapse: "collapse",
    fontSize: tokens.fontSizeBase200,
    "& th, & td": {
      padding: "6px 10px",
      textAlign: "left",
      borderBottom: `1px solid ${tokens.colorNeutralStroke2}`,
    },
    "& th": {
      fontWeight: tokens.fontWeightSemibold,
      color: tokens.colorNeutralForeground2,
    },
    "& td:last-child": {
      fontFamily: tokens.fontFamilyMonospace,
      fontSize: tokens.fontSizeBase100,
      wordBreak: "break-all",
    },
  },
  footer: {
    textAlign: "center",
    marginTop: "24px",
    paddingTop: "16px",
    borderTop: `1px solid ${tokens.colorNeutralStroke2}`,
    display: "flex",
    flexDirection: "column",
    gap: "4px",
  },
});

function App() {
  const styles = useStyles();

  const [edition, setEdition] = useState<string>("");
  const [customEditionId, setCustomEditionId] = useState("");
  const [languages, setLanguages] = useState<Sku[]>([]);
  const [language, setLanguage] = useState<string>("");
  const [links, setLinks] = useState<LinksResult | null>(null);
  const [loadingSkus, setLoadingSkus] = useState(false);
  const [loadingLinks, setLoadingLinks] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [hashDialogOpen, setHashDialogOpen] = useState(false);

  const isCustom = edition === "custom";
  const effectiveEdition = isCustom ? customEditionId : edition;
  const canSelectLanguage = languages.length > 0;
  const canGenerate = !!effectiveEdition && !!language && !loadingLinks;

  async function fetchSkus(editionKey: string) {
    setLanguages([]);
    setLanguage("");
    setLinks(null);
    setError(null);
    setLoadingSkus(true);

    try {
      const res = await fetch(
        `/api/skus?edition=${encodeURIComponent(editionKey)}`,
      );
      const data = await res.json();
      if (data.error) throw new Error(data.error);
      setLanguages(data as Sku[]);
    } catch (e: unknown) {
      setError((e as Error).message);
    } finally {
      setLoadingSkus(false);
    }
  }

  function onEditionSelect(value: string) {
    setEdition(value);
    setCustomEditionId("");
    setLanguages([]);
    setLanguage("");
    setLinks(null);
    setError(null);

    if (value !== "custom") {
      fetchSkus(value);
    }
  }

  function onCustomEditionSubmit() {
    if (customEditionId.trim()) {
      fetchSkus(customEditionId.trim());
    }
  }

  async function onGenerate() {
    if (!effectiveEdition || !language) return;
    setLinks(null);
    setError(null);
    setLoadingLinks(true);

    try {
      const res = await fetch(
        `/api/links?edition=${encodeURIComponent(effectiveEdition)}&language=${encodeURIComponent(language)}`,
      );
      const data = await res.json();
      if (data.error) throw new Error(data.error);
      setLinks(data as LinksResult);
    } catch (e: unknown) {
      setError((e as Error).message);
    } finally {
      setLoadingLinks(false);
    }
  }

  const editionLabel = EDITIONS.find((e) => e.key === edition)?.label ?? "";
  const languageLabel =
    languages.find((s) => s.Language === language)?.LocalizedLanguage ?? "";
  const hashes = links?.hashes ?? {};
  const hasHashes = Object.keys(hashes).length > 0;

  return (
    <div className={styles.root}>
      <div className={styles.container}>
        <div className={styles.header}>
          <Title2>Windows ISO Downloader</Title2>
          <br />
          <Text>Download Windows ISOs directly from Microsoft</Text>
        </div>

        {error && (
          <MessageBar intent="error">
            <MessageBarBody>{error}</MessageBarBody>
          </MessageBar>
        )}

        <Card className={styles.card}>
          <div className={styles.form}>
            <Field label="Windows Edition">
              <Dropdown
                placeholder="Select an edition..."
                value={editionLabel}
                selectedOptions={edition ? [edition] : []}
                onOptionSelect={(_, data) => {
                  if (data.optionValue) onEditionSelect(data.optionValue);
                }}
              >
                {EDITIONS.map((e) => (
                  <Option key={e.key} value={e.key} text={e.label}>
                    {e.label}
                  </Option>
                ))}
              </Dropdown>
            </Field>

            {isCustom && (
              <Field label="Product Edition ID">
                <Input
                  placeholder="e.g. 3321"
                  value={customEditionId}
                  onChange={(_, data) => setCustomEditionId(data.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") onCustomEditionSubmit();
                  }}
                  contentAfter={
                    <Button
                      appearance="transparent"
                      size="small"
                      onClick={onCustomEditionSubmit}
                      disabled={!customEditionId.trim() || loadingSkus}
                    >
                      {loadingSkus ? <Spinner size="tiny" /> : "Load"}
                    </Button>
                  }
                />
              </Field>
            )}

            {(canSelectLanguage || loadingSkus) && (
              <Field label="Language">
                {loadingSkus ? (
                  <Spinner size="small" label="Fetching languages..." />
                ) : (
                  <Dropdown
                    placeholder="Select a language..."
                    value={languageLabel}
                    selectedOptions={language ? [language] : []}
                    onOptionSelect={(_, data) => {
                      if (data.optionValue) setLanguage(data.optionValue);
                    }}
                  >
                    {languages.map((s) => (
                      <Option
                        key={s.Id}
                        value={s.Language}
                        text={`${s.LocalizedLanguage} (${s.Language})`}
                      >
                        {s.LocalizedLanguage} ({s.Language})
                      </Option>
                    ))}
                  </Dropdown>
                )}
              </Field>
            )}

            {canSelectLanguage && (
              <Button
                appearance="primary"
                icon={
                  loadingLinks ? (
                    <Spinner size="tiny" />
                  ) : (
                    <ArrowDownloadRegular />
                  )
                }
                size="large"
                disabled={!canGenerate}
                onClick={onGenerate}
              >
                {loadingLinks
                  ? "Generating download links..."
                  : "Generate Download Links"}
              </Button>
            )}
          </div>
        </Card>

        {links && (
          <Card className={styles.downloadCard}>
            <CardHeader
              image={<CheckmarkCircleRegular fontSize={24} />}
              header={<Subtitle1>{links.localizedLanguage}</Subtitle1>}
              description={links.filename}
            />
            <div className={styles.downloadItem}>
              {links.downloads.map((dl, i) => (
                <Button
                  key={i}
                  as="a"
                  href={dl.url}
                  target="_blank"
                  appearance="primary"
                  icon={<ArrowDownloadRegular />}
                  size="large"
                >
                  {dl.name}
                </Button>
              ))}
            </div>
            <div className={styles.meta}>
              {links.expiresAt && (
                <Text size={200}>Link expires: {links.expiresAt}</Text>
              )}
              {hasHashes && (
                <Link onClick={() => setHashDialogOpen(true)} inline>
                  <ShieldCheckmarkRegular fontSize={14} /> View SHA-256 hashes
                </Link>
              )}
            </div>
          </Card>
        )}

        {/* Hash dialog */}
        <Dialog
          open={hashDialogOpen}
          onOpenChange={(_, data) => setHashDialogOpen(data.open)}
        >
          <DialogSurface>
            <DialogBody>
              <DialogTitle
                action={
                  <DialogTrigger action="close">
                    <Button
                      appearance="subtle"
                      icon={<DismissRegular />}
                      aria-label="Close"
                    />
                  </DialogTrigger>
                }
              >
                SHA-256 Hashes
              </DialogTitle>
              <DialogContent>
                <table className={styles.hashTable}>
                  <thead>
                    <tr>
                      <th>File</th>
                      <th>SHA-256</th>
                    </tr>
                  </thead>
                  <tbody>
                    {Object.entries(hashes).map(([key, hash]) => (
                      <tr key={key}>
                        <td>{key}</td>
                        <td>{hash}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </DialogContent>
            </DialogBody>
          </DialogSurface>
        </Dialog>

        {/* Footer */}
        <div className={styles.footer}>
          <Body1>
            Licensed under{" "}
            <Link
              href="https://www.gnu.org/licenses/gpl-3.0.html"
              target="_blank"
              inline
            >
              GPL-3.0
            </Link>
          </Body1>
          <Text size={200}>
            <Link href="https://wisodocs.krnl64.win" target="_blank" inline>
              Docs
            </Link>
            {" — "}
            <Link
              href="https://github.com/ntkrnl64/wisodown"
              target="_blank"
              inline
            >
              Source code on GitHub
            </Link>
            {" — "}
            ISOs are downloaded directly from Microsoft servers.
          </Text>
        </div>
      </div>
    </div>
  );
}

export default App;
