"use client";
import { Button, Card, Spinner, Tabs } from "@heroui/react";
import { Activity, Check, Clock3, Trash2 } from "lucide-react";
import { useState } from "react";
import { usePolling } from "@/hooks/use-polling";
import { errorMessage, request } from "@/lib/api";
import { dateLabel, timeLabel } from "@/lib/format";
import type { Feedback, RunDetail as Detail } from "@/lib/types";
import { ActivityLog } from "./activity-log";
import { Notice } from "./notice";
import { SourceReview } from "./source-review";
import { StatusBadge } from "./status-badge";
export function RunDetail({
  id,
  onChange,
  onDeleted,
}: {
  id: string;
  onChange: () => void;
  onDeleted: () => void;
}) {
  const state = usePolling<Detail>(`/api/runs/${id}`, 1500);
  const [draft, setDraft] = useState<Feedback | null>(null);
  const [pending, setPending] = useState<string | null>(null);
  const [error, setError] = useState<string>();
  const d = state.data;
  const feedback = draft || d?.feedback || { overall: "", items: {} };
  async function decide(action: string, no_feedback = false) {
    setPending(action);
    try {
      const r = await request<{ status: string }>(`/api/runs/${id}/feedback`, {
        method: "POST",
        body: JSON.stringify({
          action,
          overall: feedback.overall,
          items: feedback.items,
          no_feedback,
        }),
      });
      state.update((x) => ({ ...x, run: { ...x.run, status: r.status } }));
      onChange();
      state.refresh();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setPending(null);
    }
  }
  if (!d)
    return (
      <div className="p-12">
        <Spinner />
      </div>
    );
  const r = d.run;
  return (
    <div className="page-content mx-auto w-full max-w-7xl px-4 py-8 sm:px-8">
      <div className="flex justify-between gap-4">
        <div>
          <span className="eyebrow text-xs tracking-widest text-muted">
            THUMBNAIL RUN
          </span>
          <h1 className="mt-2 font-display text-3xl font-semibold">
            {r.topic}
          </h1>
          {r.summary && (
            <p className="mt-2 max-w-3xl text-sm text-muted">{r.summary}</p>
          )}
          <div className="mt-3 flex gap-3">
            <StatusBadge status={r.status} />
            <span className="flex items-center gap-1 text-xs text-muted">
              <Clock3 size={14} />
              {dateLabel(r.created_at)} · {timeLabel(r.created_at)}
            </span>
          </div>
        </div>
        <Button
          isIconOnly
          variant="ghost"
          aria-label="Delete run"
          onPress={async () => {
            await request(`/api/runs/${id}`, { method: "DELETE" });
            onDeleted();
          }}
        >
          <Trash2 />
        </Button>
      </div>
      {r.error && <Notice>{r.error}</Notice>}
      {error && <Notice>{error}</Notice>}
      <Tabs defaultSelectedKey="review" className="mt-8">
        <Tabs.ListContainer>
          <Tabs.List aria-label="Run views">
            <Tabs.Tab id="review">
              <Check size={16} />
              Review
              <Tabs.Indicator />
            </Tabs.Tab>
            <Tabs.Tab id="activity">
              <Activity size={16} />
              Activity
              <Tabs.Indicator />
            </Tabs.Tab>
          </Tabs.List>
        </Tabs.ListContainer>
        <Tabs.Panel id="review">
          <SourceReview
            detail={d}
            draft={feedback}
            onDraft={setDraft}
            onDecision={decide}
            pending={pending}
          />
        </Tabs.Panel>
        <Tabs.Panel id="activity">
          <ActivityLog messages={d.messages} />
        </Tabs.Panel>
      </Tabs>
    </div>
  );
}
