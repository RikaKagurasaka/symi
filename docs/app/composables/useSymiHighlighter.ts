import { createGlobalState } from "@vueuse/core";
import { createHighlighter } from "shiki";
import {
  symiLang,
  symiThemeDark,
  symiThemeLight,
} from "../assets/symi-textmate";

export const useSymiHighlighter = createGlobalState(() =>
  createHighlighter({
    themes: [symiThemeLight as any, symiThemeDark as any],
    langs: [symiLang],
  }),
);
