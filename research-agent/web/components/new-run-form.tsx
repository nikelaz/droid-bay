"use client";

import {
  Autocomplete,
  Button,
  Card,
  Description,
  EmptyState,
  FieldError,
  Input,
  Label,
  ListBox,
  SearchField,
  Spinner,
  TextArea,
  TextField,
  useFilter,
} from "@heroui/react";
import {
  ArrowRight,
  CheckCheck,
  FileSearch,
  RefreshCw,
  Sparkles,
} from "lucide-react";
import { useState } from "react";
import { usePolling } from "@/hooks/use-polling";
import { errorMessage, request } from "@/lib/api";
import { groupModels, providerLabel } from "@/lib/format";
import type { Model, ModelCatalog, NewRun } from "@/lib/types";
import { Notice } from "./notice";

const stages = [
  {
    key: "model_research",
    label: "Researcher",
    description: "Finds and annotates sources",
    icon: FileSearch,
  },
  {
    key: "model_critique",
    label: "Reviewer",
    description: "Checks relevance and quality",
    icon: CheckCheck,
  },
  {
    key: "model_fix",
    label: "Refiner",
    description: "Improves sources with feedback",
    icon: Sparkles,
  },
] as const;

/// A single-choice dropdown with a search field, for catalogs that can hold
/// hundreds of models.
function ModelAutocomplete({
  stage,
  value,
  models,
  disabled,
  onChange,
}: {
  stage: (typeof stages)[number];
  value: string;
  models: Model[];
  disabled: boolean;
  onChange: (id: string) => void;
}) {
  const { contains } = useFilter({ sensitivity: "base" });
  const name = `${stage.label} model`;
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
        textValue={model.display_name || model.model}
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
      aria-label={name}
      value={value || null}
      onChange={(next) => onChange(next == null ? "" : String(next))}
      isDisabled={disabled}
      placeholder="Choose model"
    >
      <Label>{stage.label}</Label>
      <Autocomplete.Trigger aria-label={name}>
        <Autocomplete.Value />
        <Autocomplete.Indicator />
      </Autocomplete.Trigger>
      <Autocomplete.Popover>
        <Autocomplete.Filter filter={contains}>
          <SearchField autoFocus aria-label={`Search ${name} models`}>
            <SearchField.Group>
              <SearchField.SearchIcon />
              <SearchField.Input placeholder="Search models…" />
              <SearchField.ClearButton />
            </SearchField.Group>
          </SearchField>
          <ListBox
            aria-label={name}
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
  const catalog = usePolling<ModelCatalog>("/api/models", 300_000);
  const [topic, setTopic] = useState("");
  const [summary, setSummary] = useState("");
  const [choices, setChoices] = useState<Record<string, string>>({});
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string>();
  const models = catalog.data?.models || [];
  const defaultModel =
    models.find((model) => model.is_default)?.id || models[0]?.id || "";
  const modelFor = (key: string) =>
    models.some((model) => model.id === choices[key])
      ? choices[key]
      : defaultModel;

  async function submit(event: React.FormEvent) {
    event.preventDefault();
    if (!topic.trim() || busy) return;
    setBusy(true);
    setError(undefined);
    const body: NewRun = {
      topic: topic.trim(),
      summary: summary.trim(),
      model_research: modelFor("model_research"),
      model_critique: modelFor("model_critique"),
      model_fix: modelFor("model_fix"),
    };
    try {
      const result = await request<{ id: string }>("/api/runs", {
        method: "POST",
        body: JSON.stringify(body),
      });
      onCreated(result.id);
    } catch (error) {
      setError(errorMessage(error));
      setBusy(false);
    }
  }

  return (
    <div className="new-research page-content mx-auto w-full max-w-4xl px-4 py-8 sm:px-8 lg:py-11">
      <div className="new-research-grid">
        <form
          onSubmit={submit}
          className="research-form flex min-w-0 flex-col gap-5"
        >
          <Card className="panel form-section">
            <div className="section-heading flex items-start gap-3">
              <span className="section-number grid size-8 shrink-0 place-items-center rounded-md bg-default text-xs font-medium text-muted">
                01
              </span>
              <div>
                <h2 className="font-display text-base font-semibold">
                  Set the direction
                </h2>
                <p className="mt-1 text-xs leading-5 text-muted">
                  A good question is the start of good research.
                </p>
              </div>
            </div>
            <TextField
              fullWidth
              isRequired
              name="topic"
              value={topic}
              onChange={setTopic}
              isDisabled={busy}
              validate={(value) =>
                !value.trim() ? "Enter a research topic." : null
              }
            >
              <Label>Research topic</Label>
              <Input placeholder="What would you like to understand?" />
              <FieldError />
            </TextField>
            <TextField
              fullWidth
              name="summary"
              value={summary}
              onChange={setSummary}
              isDisabled={busy}
            >
              <Label>
                Focus & context
                <span className="optional ml-2 text-xs font-normal text-muted">
                  Optional
                </span>
              </Label>
              <TextArea
                rows={5}
                placeholder="Add a specific angle, questions to answer, or the kinds of sources you’re looking for."
              />
              <Description>
                The more context you give, the more focused your results.
              </Description>
            </TextField>
          </Card>
          <Card className="panel form-section">
            <div className="section-heading flex items-start gap-3">
              <span className="section-number grid size-8 shrink-0 place-items-center rounded-md bg-default text-xs font-medium text-muted">
                02
              </span>
              <div>
                <h2 className="font-display text-base font-semibold">
                  Choose your research team
                </h2>
                <p className="mt-1 text-xs leading-5 text-muted">
                  Use the recommended model or tailor each role.
                </p>
              </div>
              <Button
                className="ml-auto shrink-0"
                aria-label="Refresh models"
                size="sm"
                variant="ghost"
                isIconOnly
                isDisabled={catalog.loading}
                onPress={catalog.refresh}
              >
                <RefreshCw
                  size={16}
                  className={catalog.loading ? "spin" : ""}
                />
              </Button>
            </div>
            {catalog.error && (
              <Notice>Models are unavailable. {catalog.error.message}</Notice>
            )}
            {catalog.data?.warning && (
              <Notice warning>{catalog.data.warning}</Notice>
            )}
            {!catalog.data && !catalog.error && (
              <p className="loading-line flex items-center gap-2 text-sm text-muted">
                <Spinner size="sm" /> Finding available models…
              </p>
            )}
            <div className="model-grid grid gap-6 sm:grid-cols-3">
              {stages.map((stage) => (
                <div className="model-role min-w-0" key={stage.key}>
                  <span className="role-icon mb-3 grid size-9 place-items-center rounded-lg bg-accent-soft text-accent">
                    <stage.icon size={19} />
                  </span>
                  <ModelAutocomplete
                    stage={stage}
                    value={modelFor(stage.key)}
                    models={models}
                    disabled={busy || !models.length}
                    onChange={(id) =>
                      setChoices((previous) => ({
                        ...previous,
                        [stage.key]: id,
                      }))
                    }
                  />
                  <p className="mt-2 text-xs leading-5 text-muted">
                    {stage.description}
                  </p>
                </div>
              ))}
            </div>
          </Card>
          {error && <Notice>{error}</Notice>}
          <div className="form-actions flex justify-end">
            <Button
              type="submit"
              className="start-button w-full sm:w-auto"
              isDisabled={
                !topic.trim() || !models.length || !!catalog.error || busy
              }
            >
              {busy ? <Spinner size="sm" /> : <Sparkles size={17} />}
              {busy ? "Starting research…" : "Start research"}
              {!busy && <ArrowRight size={17} />}
            </Button>
          </div>
        </form>
      </div>
    </div>
  );
}
