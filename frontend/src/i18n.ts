import { useMemo, useState } from "react";

export type Locale = "en" | "zh";

const translations = {
  en: {
    title: "Windows ISO Downloader",
    subtitle: "Download Windows ISOs directly from Microsoft",
    editionLabel: "Windows Edition",
    editionPlaceholder: "Select an edition...",
    editionX64: "Windows 11 (x64)",
    editionArm64: "Windows 11 (ARM64)",
    editionWin10: "Windows 10",
    editionCnHome: "Windows 11 Home China",
    editionCnPro: "Windows 11 Pro China",
    editionCustom: "Custom Edition ID...",
    productEditionId: "Product Edition ID",
    productEditionPlaceholder: "e.g. 3321",
    load: "Load",
    languageLabel: "Language",
    fetchingLanguages: "Fetching languages...",
    languagePlaceholder: "Select a language...",
    generateLinks: "Generate Download Links",
    generatingLinks: "Generating download links...",
    linkExpires: "Link expires: {time}",
    viewHashes: "View SHA-256 hashes",
    hashDialogTitle: "SHA-256 Hashes",
    fileHeader: "File",
    sha256Header: "SHA-256",
    licensedUnder: "Licensed under",
    docs: "Docs",
    sourceCode: "Source code on GitHub",
    footerNote: "ISOs are downloaded directly from Microsoft servers.",
  },
  zh: {
    title: "Windows ISO \u4E0B\u8F7D\u5668",
    subtitle:
      "\u76F4\u63A5\u4ECE\u5FAE\u8F6F\u670D\u52A1\u5668\u4E0B\u8F7D Windows ISO",
    editionLabel: "Windows \u7248\u672C",
    editionPlaceholder: "\u9009\u62E9\u7248\u672C...",
    editionX64: "Windows 11 (x64)",
    editionArm64: "Windows 11 (ARM64)",
    editionWin10: "Windows 10",
    editionCnHome: "Windows 11 \u5BB6\u5EAD\u4E2D\u6587\u7248",
    editionCnPro: "Windows 11 \u4E13\u4E1A\u4E2D\u6587\u7248",
    editionCustom: "\u81EA\u5B9A\u4E49\u7248\u672C ID...",
    productEditionId: "\u4EA7\u54C1\u7248\u672C ID",
    productEditionPlaceholder: "\u4F8B\u5982 3321",
    load: "\u52A0\u8F7D",
    languageLabel: "\u8BED\u8A00",
    fetchingLanguages: "\u6B63\u5728\u83B7\u53D6\u8BED\u8A00\u5217\u8868...",
    languagePlaceholder: "\u9009\u62E9\u8BED\u8A00...",
    generateLinks: "\u751F\u6210\u4E0B\u8F7D\u94FE\u63A5",
    generatingLinks: "\u6B63\u5728\u751F\u6210\u4E0B\u8F7D\u94FE\u63A5...",
    linkExpires: "\u94FE\u63A5\u8FC7\u671F\u65F6\u95F4\uFF1A{time}",
    viewHashes: "\u67E5\u770B SHA-256 \u54C8\u5E0C\u503C",
    hashDialogTitle: "SHA-256 \u54C8\u5E0C\u503C",
    fileHeader: "\u6587\u4EF6",
    sha256Header: "SHA-256",
    licensedUnder: "\u8BB8\u53EF\u8BC1",
    docs: "\u6587\u6863",
    sourceCode: "GitHub \u6E90\u4EE3\u7801",
    footerNote:
      "ISO \u6587\u4EF6\u76F4\u63A5\u4ECE\u5FAE\u8F6F\u670D\u52A1\u5668\u4E0B\u8F7D\u3002",
  },
} as const satisfies Record<Locale, Record<string, string>>;

export type Translations = { readonly [K in keyof (typeof translations)["en"]]: string };

export function detectLocale(): Locale {
  const lang = navigator.language.toLowerCase();
  if (lang.startsWith("zh")) return "zh";
  return "en";
}

export function useLocale() {
  const [locale, setLocale] = useState<Locale>(detectLocale);
  const t = useMemo(() => translations[locale], [locale]);
  return { locale, setLocale, t } as const;
}
