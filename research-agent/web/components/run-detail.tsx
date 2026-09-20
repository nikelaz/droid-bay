"use client";

import { Button, Card, Modal, Spinner, Tabs } from "@heroui/react";
import {
  Activity,
  BookOpen,
  Check,
  ChevronRight,
  Clock3,
  Trash2,
} from "lucide-react";
import { useState } from "react";
import { usePolling } from "@/hooks/use-polling";
import { ApiError, errorMessage, request } from "@/lib/api";
import { dateLabel, providerLabel, splitModel, timeLabel } from "@/lib/format";
import type { Feedback, RunDetail as Detail } from "@/lib/types";
import { ActivityLog } from "./activity-log";
import { SourceReview } from "./source-review";
import { StatusBadge } from "./status-badge";
import { Notice } from "./notice";

export function RunDetail({
  id,
  onChange,
  onDeleted,
}: {
  id: string;
  onChange: () => void;
  onDeleted: () => void;
}) {
  const state = usePolling<Detail>(`/api/runs/${encodeURIComponent(id)}`, 1500);
  const [draft, setDraft] = useState<Feedback | null>(null);
  const [pending, setPending] = useState<string | null>(null);
  const [error, setError] = useState<string>();
  const [deleting, setDeleting] = useState(false);
  const detail = state.data;
  const feedback = draft || detail?.feedback || { overall: "", sources: {} };

  async function decide(action: "accept" | "revise") {
    if (pending) return;
    if (
      action === "revise" &&
      !feedback.overall.trim() &&
      !Object.values(feedback.sources).some((value) => value.trim())
    ) {
      setError(
        "Add overall or per-source feedback before requesting a revision.",
      );
      return;
    }
    setPending(action);
    setError(undefined);
    try {
      const result = await request<{ status: string }>(
        `/api/runs/${encodeURIComponent(id)}/feedback`,
        { method: "POST", body: JSON.stringify({ action, ...feedback }) },
      );
      state.update((detail) => ({
        ...detail,
        run: { ...detail.run, status: result.status },
      }));
      state.refresh();
      onChange();
    } catch (error) {
      setError(errorMessage(error));
    } finally {
      setPending(null);
    }
  }
  async function remove() {
    setPending("delete");
    setError(undefined);
    try {
      await request(`/api/runs/${encodeURIComponent(id)}`, {
        method: "DELETE",
      });
      onDeleted();
    } catch (error) {
      setError(errorMessage(error));
      setDeleting(false);
      setPending(null);
    }
  }

  if (state.error instanceof ApiError && state.error.status === 404)
    return (
      <div className="page-content mx-auto w-full max-w-7xl px-4 py-8 sm:px-8 lg:py-11">
        <Notice>This research run no longer exists.</Notice>
        <Button variant="secondary" onPress={onDeleted}>
          Back to workspace
        </Button>
      </div>
    );
  if (!detail)
    return (
      <div className="page-content mx-auto flex w-full max-w-7xl items-center justify-center gap-3 px-4 py-24 text-muted sm:px-8">
        {state.error ? (
          <Notice>
            {state.error.message}
            <Button variant="tertiary" onPress={state.refresh}>
              Retry
            </Button>
          </Notice>
        ) : (
          <div className="loading-state flex items-center gap-3">
            <Spinner />
            <p>Opening your research…</p>
          </div>
        )}
      </div>
    );
  const { run } = detail;
  const step =
    run.status === "queued"
      ? -1
      : run.status === "running_research"
        ? 0
        : ["running_critique", "running_fix"].includes(run.status)
          ? 1
          : run.status === "error"
            ? -1
            : 2;
  return (
    <div className="page-content run-detail mx-auto w-full max-w-7xl px-4 py-8 sm:px-8 lg:py-11">
      <div className="run-heading flex justify-between gap-4">
        <div>
          <div className="eyebrow text-xs font-medium tracking-widest text-muted">
            RESEARCH WORKSPACE
          </div>
          <h1 className="mt-3 max-w-4xl break-words font-display text-2xl font-semibold tracking-tight sm:text-3xl">
            {run.topic}
          </h1>
          {run.summary && (
            <p className="run-summary mt-3 max-w-3xl whitespace-pre-wrap text-sm leading-7 text-muted">
              {run.summary}
            </p>
          )}
          <div className="run-metadata mt-4 flex flex-wrap items-center gap-4">
            <StatusBadge status={run.status} />
            <span className="flex items-center gap-2 text-xs text-muted">
              <Clock3 size={14} />
              {dateLabel(run.created_at)} · {timeLabel(run.created_at)}
            </span>
          </div>
        </div>
        <Button
          variant="ghost"
          isIconOnly
          aria-label="Delete research"
          onPress={() => setDeleting(true)}
        >
          <Trash2 size={18} />
        </Button>
      </div>
      <ol
        className="pipeline my-6 flex items-center justify-between gap-3 overflow-x-auto border-y border-border py-5"
        aria-label="Research progress"
      >
        {["Research", "Review & refine", "Your review"].map((label, index) => (
          <li
            key={label}
            className={`flex shrink-0 items-center gap-2 text-xs ${index <= step ? "text-accent" : "text-muted"}`}
            aria-current={index === step ? "step" : undefined}
          >
            <span
              className={`step-circle grid size-6 place-items-center rounded-full text-xs ${index <= step ? "bg-accent-soft text-accent" : "bg-default text-muted"}`}
            >
              {index < step || run.status === "accepted" ? (
                <Check size={13} />
              ) : (
                index + 1
              )}
            </span>
            <span>{label}</span>
            {index < 2 && (
              <ChevronRight className="ml-3 text-muted" size={16} />
            )}
          </li>
        ))}
      </ol>
      {state.error && (
        <Notice warning>
          Live updates paused: {state.error.message} Retrying automatically.
        </Notice>
      )}
      {run.error && <Notice>{run.error}</Notice>}
      {error && <Notice>{error}</Notice>}
      <Tabs defaultSelectedKey="sources" className="detail-tabs w-full">
        <Tabs.ListContainer>
          <Tabs.List aria-label="Research views">
            <Tabs.Tab id="sources">
              <BookOpen size={16} />
              Sources
              <span className="tab-count rounded-md bg-accent-soft px-1.5 text-xs text-muted">
                {detail.result?.sources.length || 0}
              </span>
              <Tabs.Indicator />
            </Tabs.Tab>
            <Tabs.Tab id="activity">
              <Activity size={16} />
              Activity
              <Tabs.Indicator />
            </Tabs.Tab>
            <Tabs.Tab id="team">
              Research team
              <Tabs.Indicator />
            </Tabs.Tab>
          </Tabs.List>
        </Tabs.ListContainer>
        <Tabs.Panel id="sources">
          <SourceReview
            detail={detail}
            draft={feedback}
            onDraft={(draft) => {
              setDraft(draft);
              setError(undefined);
            }}
            onDecision={decide}
            pending={pending}
          />
        </Tabs.Panel>
        <Tabs.Panel id="activity">
          <ActivityLog messages={detail.messages} />
        </Tabs.Panel>
        <Tabs.Panel id="team">
          <div className="content-heading mb-6">
            <div>
              <h2 className="font-display text-lg font-semibold">
                The models behind this research
              </h2>
              <p className="mt-1 text-xs leading-5 text-muted">
                Each role brings a different perspective to your question.
              </p>
            </div>
          </div>
          <div className="team-grid grid gap-5 md:grid-cols-3">
            {[
              [
                "Researcher",
                run.model_research,
                "Discovers and annotates sources.",
              ],
              ["Reviewer", run.model_critique, "Checks quality and relevance."],
              ["Refiner", run.model_fix, "Applies critique and your feedback."],
            ].map(([label, model, description]) => {
              const selection = splitModel(model);
              return (
                <Card key={label} className="panel team-card">
                  <span className="eyebrow text-xs font-medium tracking-widest text-muted">
                    {label}
                  </span>
                  <h3 className="mt-3 break-words font-display text-lg font-semibold">
                    {selection.model || model}
                  </h3>
                  {selection.provider && (
                    <span className="provider-tag mt-1 inline-block text-xs text-muted">
                      {providerLabel(selection.provider)}
                    </span>
                  )}
                  <p className="text-xs text-muted">{description}</p>
                </Card>
              );
            })}
          </div>
        </Tabs.Panel>
      </Tabs>
      <Modal
        isOpen={deleting}
        onOpenChange={(open) => {
          if (!pending) setDeleting(open);
        }}
      >
        <Modal.Backdrop>
          <Modal.Container>
            <Modal.Dialog>
              <Modal.Header>
                <Modal.Heading>Delete this research?</Modal.Heading>
              </Modal.Header>
              <Modal.Body>
                <p>
                  “{run.topic}” and its saved history will be removed. This
                  cannot be undone.
                </p>
              </Modal.Body>
              <Modal.Footer>
                <Button
                  variant="secondary"
                  isDisabled={pending !== null}
                  onPress={() => setDeleting(false)}
                >
                  Keep research
                </Button>
                <Button
                  variant="danger"
                  isDisabled={pending !== null}
                  onPress={remove}
                >
                  {pending === "delete" && <Spinner size="sm" />}Delete research
                </Button>
              </Modal.Footer>
            </Modal.Dialog>
          </Modal.Container>
        </Modal.Backdrop>
      </Modal>
    </div>
  );
}
