export const darkTheme = {
  bg: "#0E1223",
  card: "#1A1E2F",
  border: "#272F42",
  muted: "#94A3B8",
  foreground: "#F8FAFC",
  mutedBg: "#272F42",
  gridLine: "#272F42",
  tabActiveBg: "#272F42",
  tabActiveText: "#F8FAFC",
  tabInactiveText: "#64748B",
  accentGreen: "#22C55E",
} as const;

export const lightTheme = {
  bg: "#FAFBFC",
  card: "#FFFFFF",
  border: "#E2E8F0",
  muted: "#94A3B8",
  foreground: "#1E293B",
  mutedBg: "#F1F5F9",
  gridLine: "#F1F5F9",
  tabActiveBg: "#FFFFFF",
  tabActiveText: "#1E293B",
  tabInactiveText: "#94A3B8",
  accentGreen: "#16A34A",
} as const;

export const colorPool = {
  dark: ["#6366f1", "#22C55E", "#F59E0B", "#EC4899", "#06B6D4", "#F97316", "#8B5CF6", "#14B8A6", "#EF4444", "#64748B"],
  light: ["#4F46E5", "#16A34A", "#D97706", "#DB2777", "#0891B2", "#EA580C", "#7C3AED", "#0D9488", "#DC2626", "#475569"],
} as const;

export function getLineStyle(index: number): "solid" | "dashed" | [number, number] {
  if (index < 3) return "solid";
  if (index < 6) return "dashed";
  return [2, 3];
}

export function getModelDisplayName(model: string, aliases: Record<string, string>): string {
  return aliases[model] || model;
}

export type ThemeTokens = {
  [K in keyof typeof darkTheme]: string;
};
