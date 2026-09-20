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
  channel_url: string;
  assets: string[];
  concept_count: number;
  model_research: string;
  model_critique: string;
  model_fix: string;
  model_image: string;
  generate_images: boolean;
  status: string;
  error: string | null;
}
export interface Concept {
  id: string;
  title?: string;
  prompt: string;
  rationale?: string;
}
export interface Thumbnail {
  id: string;
  title?: string;
  prompt?: string;
  rationale?: string;
  image_url?: string;
}
export interface Feedback {
  overall: string;
  items: Record<string, string>;
}
export interface Message {
  id: number;
  created_at: number;
  phase: string;
  role: string;
  content: string;
}
export interface RunDetail {
  run: Run;
  result: { concepts?: Concept[]; thumbnails?: Thumbnail[] } | null;
  messages: Message[];
  feedback: Feedback | null;
}
export interface NewRun {
  topic: string;
  summary: string;
  channel_url: string;
  assets: string[];
  concept_count: number;
  model_research: string;
  model_critique: string;
  model_fix: string;
  model_image: string;
  generate_images: boolean;
}
