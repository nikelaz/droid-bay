export interface Model {
  id: string;
  provider: string;
  model: string;
  display_name: string;
  is_default: boolean;
}
export interface ModelCatalog {
  models: Model[];
  stale: boolean;
  warning: string | null;
}
export interface Run {
  id: string;
  created_at: number;
  updated_at: number;
  topic: string;
  summary: string;
  model_research: string;
  model_critique: string;
  model_fix: string;
  status: string;
  error: string | null;
}
export interface Source {
  id: string;
  title?: unknown;
  url?: unknown;
  authors?: unknown;
  date?: unknown;
  summary?: unknown;
  relevance?: unknown;
}
export interface Message {
  id: number;
  created_at: number;
  phase: string;
  role: string;
  content: string;
}
export interface Feedback {
  overall: string;
  sources: Record<string, string>;
}
export interface RunDetail {
  run: Run;
  result: { sources: Source[] } | null;
  messages: Message[];
  feedback: Feedback | null;
}
export interface NewRun {
  topic: string;
  summary: string;
  model_research: string;
  model_critique: string;
  model_fix: string;
}
