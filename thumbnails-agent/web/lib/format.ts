export const statusLabels: Record<string, string> = {
  queued: "Queued",
  running_concepts: "Generating concepts",
  running_critique: "Critiquing concepts",
  running_refine: "Refining concepts",
  concept_review: "Concept review",
  running_images: "Generating images",
  image_review: "Image review",
  prompts_ready: "Prompts ready",
  accepted: "Accepted",
  error: "Failed",
};
const providerNames: Record<string, string> = {
  codex: "Codex",
  opencode: "OpenCode",
  mock: "Mock",
};
export const providerLabel = (provider: string) =>
  providerNames[provider] || provider;
export function splitModel(selection: string): {
  provider: string;
  model: string;
} {
  const separator = selection.indexOf("::");
  if (separator === -1) return { provider: "", model: selection };
  return {
    provider: selection.slice(0, separator),
    model: selection.slice(separator + 2),
  };
}
export function groupModels<T extends { provider: string }>(
  models: T[],
): { provider: string; models: T[] }[] {
  const groups: { provider: string; models: T[] }[] = [];
  for (const model of models) {
    const group = groups.find((entry) => entry.provider === model.provider);
    if (group) group.models.push(model);
    else groups.push({ provider: model.provider, models: [model] });
  }
  return groups;
}
export const dateLabel = (seconds: number) =>
  new Date(seconds * 1000).toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
  });
export const timeLabel = (seconds: number) =>
  new Date(seconds * 1000).toLocaleTimeString("en-US", {
    hour: "2-digit",
    minute: "2-digit",
  });
export function safeUrl(url: unknown): string | undefined {
  if (typeof url !== "string") return undefined;
  try {
    const parsed = new URL(url);
    return ["http:", "https:"].includes(parsed.protocol)
      ? parsed.href
      : undefined;
  } catch {
    return undefined;
  }
}

// Model-generated bibliography fields are not guaranteed to be plain strings.
export function displayText(value: unknown): string {
  if (value === null || value === undefined) return "";
  if (typeof value === "string") return value;
  if (Array.isArray(value)) return value.map(displayText).join(", ");
  if (typeof value === "object") return JSON.stringify(value);
  return String(value);
}
