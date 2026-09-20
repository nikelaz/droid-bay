"use client";
import {
  Autocomplete,
  EmptyState,
  ListBox,
  SearchField,
  useFilter,
  Button,
  Card,
  Input,
  Label,
  Spinner,
  Switch,
  TextArea,
  TextField,
} from "@heroui/react";
import { ClipboardCopy, ImagePlus, Sparkles } from "lucide-react";
import { useState } from "react";
import { usePolling } from "@/hooks/use-polling";
import { errorMessage, request } from "@/lib/api";
import { groupModels, providerLabel } from "@/lib/format";
import type { Model, ModelCatalog, NewRun } from "@/lib/types";
import { Notice } from "./notice";
const roles = [
  {
    key: "model_research",
    label: "Concept creator",
    description: "Develops thumbnail prompts from your video and channel.",
  },
  {
    key: "model_critique",
    label: "Critic",
    description: "Reviews the concepts and recommends improvements.",
  },
  {
    key: "model_fix",
    label: "Refiner",
    description: "Applies critique and your concept feedback.",
  },
  {
    key: "model_image",
    label: "Image generator",
    description: "Runs the image stage and applies thumbnail feedback.",
  },
] as const;
function ModelPicker({
  label,
  value,
  models,
  disabled,
  onChange,
}: {
  label: string;
  value: string;
  models: Model[];
  disabled: boolean;
  onChange: (id: string) => void;
}) {
  const { contains } = useFilter({ sensitivity: "base" });
  const items = groupModels(models).flatMap((group) => [
    <ListBox.Item
      id={`__provider__${group.provider}`}
      key={`__provider__${group.provider}`}
      textValue={`${providerLabel(group.provider)} models`}
      isDisabled
    >
      <Label className="provider-heading">
        {providerLabel(group.provider)} models
      </Label>
    </ListBox.Item>,
    ...group.models.map((model) => (
      <ListBox.Item
        id={model.id}
        key={model.id}
        textValue={`${model.display_name || model.model} ${model.model} ${providerLabel(model.provider)}`}
      >
        <Label>{model.display_name || model.model}</Label>
        <ListBox.ItemIndicator />
      </ListBox.Item>
    )),
  ]);
  return (
    <Autocomplete
      fullWidth
      isRequired
      selectionMode="single"
      aria-label={`${label} model`}
      value={value || null}
      onChange={(next) => onChange(next == null ? "" : String(next))}
      isDisabled={disabled}
      placeholder="Choose model"
    >
      <Label>{label}</Label>
      <Autocomplete.Trigger aria-label={`${label} model`}>
        <Autocomplete.Value />
        <Autocomplete.Indicator />
      </Autocomplete.Trigger>
      <Autocomplete.Popover>
        <Autocomplete.Filter filter={contains}>
          <SearchField autoFocus aria-label={`Search ${label} models`}>
            <SearchField.Group>
              <SearchField.SearchIcon />
              <SearchField.Input placeholder="Search models…" />
              <SearchField.ClearButton />
            </SearchField.Group>
          </SearchField>
          <ListBox
            aria-label={`${label} model`}
            renderEmptyState={() => (
              <EmptyState>No models match your search</EmptyState>
            )}
          >
            {items}
          </ListBox>
        </Autocomplete.Filter>
      </Autocomplete.Popover>
    </Autocomplete>
  );
}
export function NewRunForm({ onCreated }: { onCreated: (id: string) => void }) {
  const cat = usePolling<ModelCatalog>("/api/models", 300000);
  const [topic, setTopic] = useState("");
  const [summary, setSummary] = useState("");
  const [channel, setChannel] = useState("");
  const [count, setCount] = useState(3);
  const [generateImages, setGenerateImages] = useState(true);
  const [assets, setAssets] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string>();
  const [choices, setChoices] = useState<Record<string, string>>({});
  const models = cat.data?.models || [];
  const model =
    cat.data?.models.find((x) => x.is_default)?.id ||
    cat.data?.models[0]?.id ||
    "";
  const modelFor = (key: string) =>
    models.some((m) => m.id === choices[key]) ? choices[key] : model;
  async function files(list: FileList | null) {
    if (!list) return;
    const next = await Promise.all(
      [...list].map(
        (f) =>
          new Promise<string>((ok, no) => {
            const r = new FileReader();
            r.onload = () => ok(String(r.result));
            r.onerror = no;
            r.readAsDataURL(f);
          }),
      ),
    );
    setAssets((a) => [...a, ...next]);
  }
  async function submit(e: React.FormEvent) {
    e.preventDefault();
    if (!topic.trim() || !model || busy || cat.error) return;
    setBusy(true);
    try {
      const body: NewRun = {
        topic,
        summary,
        channel_url: channel,
        assets,
        concept_count: count,
        model_research: modelFor("model_research"),
        model_critique: modelFor("model_critique"),
        model_fix: modelFor("model_fix"),
        model_image: generateImages ? modelFor("model_image") : "",
        generate_images: generateImages,
      };
      onCreated(
        (
          await request<{ id: string }>("/api/runs", {
            method: "POST",
            body: JSON.stringify(body),
          })
        ).id,
      );
    } catch (e) {
      setError(errorMessage(e));
      setBusy(false);
    }
  }
  return (
    <form
      onSubmit={submit}
      className="page-content mx-auto flex w-full max-w-3xl flex-col gap-5 px-4 py-8 sm:px-8"
    >
      <div>
        <span className="eyebrow text-xs tracking-widest text-muted">
          THUMBNAIL WORKSPACE
        </span>
        <h1 className="mt-2 font-display text-3xl font-semibold">
          Create a thumbnail direction
        </h1>
        <p className="mt-2 text-sm text-muted">
          Generate concepts, refine them with your feedback, then approve final
          images.
        </p>
      </div>
      <Card className="panel flex flex-col gap-5">
        <TextField isRequired value={topic} onChange={setTopic}>
          <Label>Video topic</Label>
          <Input placeholder="e.g. Why AI agents fail in production" />
        </TextField>
        <TextField value={summary} onChange={setSummary}>
          <Label>Video summary</Label>
          <TextArea
            rows={4}
            placeholder="What the video covers, audience, hook, and important details."
          />
        </TextField>
        <TextField value={channel} onChange={setChannel}>
          <Label>
            YouTube channel URL <span className="text-muted">(optional)</span>
          </Label>
          <Input placeholder="https://youtube.com/@yourchannel" />
        </TextField>
        <TextField
          value={String(count)}
          onChange={(v) => setCount(Math.max(1, Math.min(12, Number(v) || 1)))}
        >
          <Label>Number of thumbnail concepts</Label>
          <Input type="number" min="1" max="12" />
        </TextField>
        <div>
          <Label>
            <ImagePlus size={15} /> Image assets{" "}
            <span className="text-muted">(0 or more)</span>
          </Label>
          <input
            className="mt-2 block text-sm"
            type="file"
            accept="image/*"
            multiple
            onChange={(e) => files(e.target.files)}
          />
          {assets.length > 0 && (
            <div className="mt-3 flex flex-wrap gap-2">
              {assets.map((a, i) => (
                <img
                  key={a}
                  src={a}
                  alt={`Asset ${i + 1}`}
                  className="size-14 rounded object-cover"
                />
              ))}
            </div>
          )}
        </div>
      </Card>
      {error && <Notice>{error}</Notice>}
      <Card className="panel flex flex-col gap-5">
        <div className="flex items-center justify-between gap-3">
          <h2 className="font-display text-lg font-semibold">
            Your thumbnail team
          </h2>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            isDisabled={busy || cat.loading}
            onPress={cat.refresh}
          >
            Refresh models
          </Button>
        </div>
        <p className="text-sm text-muted">
          Choose a model for each role. Each selection starts with the
          provider’s default.
        </p>
        {cat.error && (
          <Notice>Models are unavailable: {cat.error.message}</Notice>
        )}
        {cat.data?.warning && <Notice warning>{cat.data.warning}</Notice>}
        {!cat.data && !cat.error && (
          <p className="flex items-center gap-2 text-sm text-muted">
            <Spinner size="sm" />
            Loading available models…
          </p>
        )}
        <div className="grid gap-5 sm:grid-cols-2">
          {roles.map((role) => {
            const hidden = role.key === "model_image" && !generateImages;
            return (
              <div key={role.key} className="min-w-0">
                <ModelPicker
                  label={role.label}
                  value={modelFor(role.key)}
                  models={models}
                  disabled={busy || !models.length || hidden}
                  onChange={(id) =>
                    setChoices((previous) => ({ ...previous, [role.key]: id }))
                  }
                />
                <p className="mt-2 break-all text-xs text-muted">
                  {hidden
                    ? "Image generation is off"
                    : modelFor(role.key) || "No model available"}
                </p>
                <p className="mt-1 text-xs text-muted">{role.description}</p>
              </div>
            );
          })}
        </div>
        <Switch
          size="sm"
          isSelected={generateImages}
          isDisabled={busy}
          onChange={setGenerateImages}
        >
          <Switch.Content>
            <Switch.Control>
              <Switch.Thumb />
            </Switch.Control>
            <Label>Generate thumbnail images</Label>
          </Switch.Content>
        </Switch>
        <p className="text-xs text-muted">
          {generateImages
            ? "Image generation requires image tools in the selected provider. This catalog lists provider models; it does not verify image-generation capabilities."
            : "Stops after concept approval. You can copy the final prompts and use them in any image tool. You can still stop at the prompts later even when generation is on."}
        </p>
      </Card>
      <Button
        type="submit"
        className="start-button self-end"
        isDisabled={busy || !topic.trim() || !model || !!cat.error}
      >
        {busy ? <Spinner size="sm" /> : <Sparkles size={17} />}
        {busy ? "Starting…" : "Generate concepts"}
      </Button>
    </form>
  );
}
