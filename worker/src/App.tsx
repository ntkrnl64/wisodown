import { useMemo, useState } from "react";
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
import { type Translations, useLocale } from "./i18n";

function getEditions(t: Translations) {
  return [
    { key: "x64", label: t.editionX64 },
    { key: "arm64", label: t.editionArm64 },
    { key: "win10", label: t.editionWin10 },
    { key: "win11-cn-home", label: t.editionCnHome },
    { key: "win11-cn-pro", label: t.editionCnPro },
    { key: "custom", label: t.editionCustom },
  ];
}

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
  localeSwitcher: {
    display: "inline-flex",
    gap: "4px",
    marginTop: "6px",
    fontSize: tokens.fontSizeBase200,
  },
  localeButton: {
    minWidth: "auto",
    padding: "0 6px",
    height: "24px",
    fontSize: tokens.fontSizeBase200,
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
  const { locale, setLocale, t } = useLocale();
  const editions = useMemo(() => getEditions(t), [t]);

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

  const editionLabel = editions.find((e) => e.key === edition)?.label ?? "";
  const languageLabel =
    languages.find((s) => s.Language === language)?.LocalizedLanguage ?? "";
  const hashes = links?.hashes ?? {};
  const hasHashes = Object.keys(hashes).length > 0;

  return (
    <div className={styles.root}>
      <div className={styles.container}>
        <div className={styles.header}>
          <Title2>{t.title}</Title2>
          <br />
          <Text>{t.subtitle}</Text>
          <div className={styles.localeSwitcher}>
            <Button
              className={styles.localeButton}
              appearance={locale === "en" ? "primary" : "subtle"}
              size="small"
              onClick={() => setLocale("en")}
            >
              EN
            </Button>
            <Button
              className={styles.localeButton}
              appearance={locale === "zh" ? "primary" : "subtle"}
              size="small"
              onClick={() => setLocale("zh")}
            >
              中文
            </Button>
          </div>
        </div>

        {error && (
          <MessageBar intent="error">
            <MessageBarBody>{error}</MessageBarBody>
          </MessageBar>
        )}

        <Card className={styles.card}>
          <div className={styles.form}>
            <Field label={t.editionLabel}>
              <Dropdown
                placeholder={t.editionPlaceholder}
                value={editionLabel}
                selectedOptions={edition ? [edition] : []}
                onOptionSelect={(_, data) => {
                  if (data.optionValue) onEditionSelect(data.optionValue);
                }}
              >
                {editions.map((e) => (
                  <Option key={e.key} value={e.key} text={e.label}>
                    {e.label}
                  </Option>
                ))}
              </Dropdown>
            </Field>

            {isCustom && (
              <Field label={t.productEditionId}>
                <Input
                  placeholder={t.productEditionPlaceholder}
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
                      {loadingSkus ? <Spinner size="tiny" /> : t.load}
                    </Button>
                  }
                />
              </Field>
            )}

            {(canSelectLanguage || loadingSkus) && (
              <Field label={t.languageLabel}>
                {loadingSkus ? (
                  <Spinner size="small" label={t.fetchingLanguages} />
                ) : (
                  <Dropdown
                    placeholder={t.languagePlaceholder}
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
                {loadingLinks ? t.generatingLinks : t.generateLinks}
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
                <Text size={200}>
                  {t.linkExpires.replace("{time}", links.expiresAt)}
                </Text>
              )}
              {hasHashes && (
                <Link onClick={() => setHashDialogOpen(true)} inline>
                  <ShieldCheckmarkRegular fontSize={14} /> {t.viewHashes}
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
                {t.hashDialogTitle}
              </DialogTitle>
              <DialogContent>
                <table className={styles.hashTable}>
                  <thead>
                    <tr>
                      <th>{t.fileHeader}</th>
                      <th>{t.sha256Header}</th>
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
            {t.licensedUnder}{" "}
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
              {t.docs}
            </Link>
            {" — "}
            <Link
              href="https://github.com/ntkrnl64/wisodown"
              target="_blank"
              inline
            >
              {t.sourceCode}
            </Link>
            {" — "}
            {t.footerNote}
          </Text>
        </div>
      </div>
    </div>
  );
}

export default App;
