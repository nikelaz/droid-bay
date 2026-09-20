"use client";

import {
  Button,
  Card,
  Label,
  Spinner,
  TextArea,
  TextField,
} from "@heroui/react";
import {
  ArrowUpRight,
  BookOpen,
  Check,
  MessageSquare,
  Send,
} from "lucide-react";
import type { Feedback, RunDetail } from "@/lib/types";
import { displayText, safeUrl } from "@/lib/format";

export function SourceReview({
  detail,
  draft,
  onDraft,
  onDecision,
  pending,
}: {
  detail: RunDetail;
  draft: Feedback;
  onDraft: (draft: Feedback) => void;
  onDecision: (action: "accept" | "revise") => void;
  pending: string | null;
}) {
  const sources = detail.result?.sources || [];
  const reviewing = detail.run.status === "review";
  if (!sources.length)
    return (
      <Card className="empty-panel panel items-center p-8 text-center sm:p-16">
        <span className="empty-icon grid size-14 place-items-center rounded-lg bg-accent-soft text-muted">
          <BookOpen size={26} />
        </span>
        <h2 className="font-display text-xl font-semibold">
          {detail.run.status === "error"
            ? "No sources to show yet"
            : "Good research takes a closer look."}
        </h2>
        <p className="max-w-md text-sm leading-7 text-muted">
          {detail.run.status === "error"
            ? "Check the activity log for details about what went wrong."
            : "Your agents are gathering and checking the evidence. Sources will appear here as the research progresses."}
        </p>
        <span className="eyebrow text-xs font-medium tracking-widest text-muted">
          FOLLOW ALONG IN THE ACTIVITY TAB
        </span>
      </Card>
    );

  return (
    <div className="review-layout grid items-start gap-6 lg:grid-cols-[minmax(0,1fr)_16rem]">
      <div className="source-list">
        <div className="content-heading mb-6 flex items-center justify-between gap-4">
          <div>
            <h2 className="font-display text-lg font-semibold">
              Annotated bibliography
            </h2>
            <p className="mt-1 text-xs leading-5 text-muted">
              {sources.length} sources, with the context that matters.
            </p>
          </div>
          <span className="count rounded-md bg-default px-2 py-1 text-xs text-muted">
            {String(sources.length).padStart(2, "0")}
          </span>
        </div>
        {sources.map((source, index) => {
          const url = safeUrl(source.url);
          return (
            <Card className="panel source-card mb-4" key={source.id}>
              <div className="source-top flex items-center justify-between gap-2">
                <span className="source-number text-xs font-medium tracking-widest text-muted">
                  SOURCE {String(index + 1).padStart(2, "0")}
                </span>
                <span className="source-domain break-words text-xs text-muted">
                  {url
                    ? new URL(url).hostname.replace(/^www\./, "")
                    : "Reference"}
                </span>
              </div>
              <h3 className="break-words font-display text-lg font-semibold leading-7">
                {url ? (
                  <a
                    className="flex justify-between gap-3 no-underline hover:text-accent hover:underline"
                    href={url}
                    target="_blank"
                    rel="noopener noreferrer"
                  >
                    {displayText(source.title) || "Untitled source"}
                    <ArrowUpRight className="mt-1 text-muted" size={18} />
                  </a>
                ) : (
                  displayText(source.title) || "Untitled source"
                )}
              </h3>
              <p className="source-meta mt-2 text-xs text-muted">
                {[source.authors, source.date]
                  .map(displayText)
                  .filter(Boolean)
                  .join(" · ") || source.id}
              </p>
              <p className="source-summary my-4 whitespace-pre-wrap break-words text-sm leading-7 text-muted">
                {displayText(source.summary)}
              </p>
              <div className="relevance border-l-2 border-accent/30 pl-3">
                <span className="text-xs font-medium tracking-wider text-muted">
                  WHY IT MATTERS
                </span>
                <p className="mt-1 text-sm leading-6 text-muted">
                  {displayText(source.relevance)}
                </p>
              </div>
              {reviewing && (
                <TextField
                  fullWidth
                  value={draft.sources[source.id] || ""}
                  onChange={(value) =>
                    onDraft({
                      ...draft,
                      sources: { ...draft.sources, [source.id]: value },
                    })
                  }
                  isDisabled={pending !== null}
                  className="source-feedback mt-5 border-t border-border pt-5"
                >
                  <Label>
                    <MessageSquare size={14} />
                    Feedback for source {index + 1}
                    <span className="optional ml-2 text-xs font-normal text-muted">
                      Optional
                    </span>
                  </Label>
                  <TextArea
                    rows={2}
                    placeholder="Something to improve or look into?"
                  />
                </TextField>
              )}
            </Card>
          );
        })}
      </div>
      <Card className="review-aside panel lg:sticky lg:top-24">
        <span className="role-icon grid size-9 place-items-center rounded-lg bg-accent-soft text-accent">
          <Check size={20} />
        </span>
        <h2 className="font-display text-lg font-semibold leading-7">
          {detail.run.status === "accepted"
            ? "Research accepted"
            : "Your perspective matters."}
        </h2>
        <p className="text-sm leading-7 text-muted">
          {detail.run.status === "accepted"
            ? "You’ve accepted this bibliography. Your sources and the full research history are saved in this workspace."
            : reviewing
              ? "Give the sources a final look. Accept the results or guide another round of refinement."
              : "Your agents are working on this bibliography. You can review and give feedback when it’s ready."}
        </p>
        {reviewing && (
          <>
            <TextField
              fullWidth
              value={draft.overall}
              onChange={(overall) => onDraft({ ...draft, overall })}
              isDisabled={pending !== null}
            >
              <Label>Overall feedback</Label>
              <TextArea
                rows={5}
                placeholder="What would make this research more useful?"
              />
            </TextField>
            <Button
              className="accept-button w-full"
              fullWidth
              isDisabled={pending !== null}
              onPress={() => onDecision("accept")}
            >
              {pending === "accept" ? (
                <Spinner size="sm" />
              ) : (
                <Check size={17} />
              )}
              Accept results
            </Button>
            <Button
              fullWidth
              variant="secondary"
              isDisabled={pending !== null}
              onPress={() => onDecision("revise")}
            >
              {pending === "revise" ? (
                <Spinner size="sm" />
              ) : (
                <Send size={15} />
              )}
              Request revision
            </Button>
            <p className="review-hint text-center text-xs text-muted">
              Add overall or per-source feedback to request a revision.
            </p>
          </>
        )}
        {detail.run.status === "accepted" && (
          <div className="accepted-note flex items-center gap-2 text-sm text-muted">
            <Check size={16} />
            Accepted and saved
          </div>
        )}
      </Card>
    </div>
  );
}
