"use client";
import {
  Button,
  Card,
  Checkbox,
  Label,
  Spinner,
  TextArea,
  TextField,
} from "@heroui/react";
import { Check, ClipboardCopy, ImageIcon, Send, Sparkles } from "lucide-react";
import { useState } from "react";
import type { Concept, Feedback, RunDetail, Thumbnail } from "@/lib/types";
import { Notice } from "./notice";

function conceptText(item: Concept | Thumbnail) {
  return [
    item.title ? `${item.title}\n` : "",
    item.prompt,
    item.rationale ? `\n\nWhy it works: ${item.rationale}` : "",
  ]
    .join("")
    .trim();
}

function allPrompts(items: (Concept | Thumbnail)[]) {
  return items
    .map((item, i) =>
      [`Thumbnail ${i + 1}`, item.title || "", item.prompt || ""]
        .filter(Boolean)
        .join("\n"),
    )
    .join("\n\n---\n\n");
}

function CopyButton({
  text,
  label,
  className,
}: {
  text: string;
  label: string;
  className?: string;
}) {
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState<string>();
  async function copy() {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setError(undefined);
      window.setTimeout(() => setCopied(false), 2000);
    } catch {
      setError("Copy failed. Select the prompt text manually.");
    }
  }
  return (
    <div className={className}>
      <Button type="button" size="sm" variant="ghost" onPress={copy}>
        <ClipboardCopy size={15} />
        {copied ? "Copied" : label}
      </Button>
      {error && <Notice>{error}</Notice>}
    </div>
  );
}

export function SourceReview({
  detail,
  draft,
  onDraft,
  onDecision,
  pending,
}: {
  detail: RunDetail;
  draft: Feedback;
  onDraft: (x: Feedback) => void;
  onDecision: (x: string, noFeedback?: boolean) => void;
  pending: string | null;
}) {
  const concepts = detail.result?.concepts || [];
  const thumbs = detail.result?.thumbnails || [];
  const conceptReview = detail.run.status === "concept_review";
  const imageReview = detail.run.status === "image_review";
  const promptsReady = detail.run.status === "prompts_ready";
  const isImages =
    thumbs.length > 0 || imageReview || detail.run.status === "accepted";
  const generateImages = detail.run.generate_images;
  const items = isImages ? thumbs : concepts;
  if (!items.length)
    return (
      <Card className="panel p-12 text-center text-muted">
        <Spinner /> <p className="mt-4">Your thumbnail team is working…</p>
      </Card>
    );
  return (
    <div className="review-layout grid items-start gap-6 lg:grid-cols-[minmax(0,1fr)_18rem]">
      <div>
        <div className="mb-5">
          <h2 className="font-display text-xl font-semibold">
            {isImages ? "Generated thumbnails" : "Thumbnail concepts"}
          </h2>
          <p className="text-sm text-muted">
            {promptsReady
              ? "These prompts are final. Copy them into your own image tool whenever you like."
              : isImages
                ? "Approve everything or comment only on the image you want changed."
                : "Review each production-ready prompt before images are generated."}
          </p>
        </div>
        {items.map((item, i) => (
          <Card key={item.id} className="panel mb-4">
            {isImages && "image_url" in item && item.image_url ? (
              <img
                src={item.image_url}
                alt={`Thumbnail ${i + 1}`}
                className="mb-4 aspect-video w-full rounded-lg object-cover"
              />
            ) : (
              <div className="mb-4 grid aspect-video place-items-center rounded-lg bg-default text-muted">
                <ImageIcon size={32} />
              </div>
            )}
            <span className="text-xs tracking-widest text-muted">
              {isImages ? "THUMBNAIL" : "CONCEPT"}{" "}
              {String(i + 1).padStart(2, "0")}
            </span>
            {!isImages && (
              <h3 className="mt-2 font-display text-lg font-semibold">
                {item.title || `Concept ${i + 1}`}
              </h3>
            )}
            <p className="mt-3 whitespace-pre-wrap text-sm leading-6 text-muted">
              {item.prompt}
            </p>
            {!isImages && (
              <div className="mt-2 flex flex-wrap items-center gap-2">
                <CopyButton text={conceptText(item)} label="Copy prompt" />
                {!conceptReview && item.rationale && (
                  <p className="border-l-2 border-accent/40 pl-3 text-sm text-muted">
                    {item.rationale}
                  </p>
                )}
              </div>
            )}
            {isImages && item.rationale && (
              <p className="mt-3 border-l-2 border-accent/40 pl-3 text-sm text-muted">
                {item.rationale}
              </p>
            )}
            {(conceptReview || imageReview) && (
              <TextField
                className="mt-5 border-t border-border pt-4"
                value={draft.items[item.id] || ""}
                onChange={(v) =>
                  onDraft({ ...draft, items: { ...draft.items, [item.id]: v } })
                }
              >
                <Label>
                  Feedback for {isImages ? "this thumbnail" : "this concept"}{" "}
                  <span className="text-muted">(optional)</span>
                </Label>
                <TextArea
                  rows={2}
                  placeholder={
                    isImages
                      ? "Only this image will be regenerated."
                      : "What should be changed?"
                  }
                />
              </TextField>
            )}
          </Card>
        ))}
      </div>
      <Card className="panel lg:sticky lg:top-24">
        <span className="role-icon grid size-9 place-items-center rounded-lg bg-accent-soft text-accent">
          {isImages ? <ImageIcon /> : <Sparkles />}
        </span>
        <h2 className="font-display text-lg font-semibold">
          {detail.run.status === "accepted"
            ? "Final images accepted"
            : promptsReady
              ? "Prompts ready"
              : isImages
                ? "Image review"
                : "Concept review"}
        </h2>
        <p className="text-sm leading-6 text-muted">
          {detail.run.status === "accepted"
            ? "Your accepted thumbnails remain available in this run."
            : promptsReady
              ? "Image generation is off. Copy the prompts into the image tool of your choice."
              : isImages
                ? "Approve the final set, or request a revision for one or more images."
                : generateImages
                  ? "Accept these prompts to generate images, send them through another refinement pass, or stop here and copy the prompts."
                  : "Accept these prompts to finish the run, or send them through another refinement pass."}
        </p>
        {(conceptReview || promptsReady) && concepts.length > 0 && (
          <CopyButton
            text={allPrompts(concepts)}
            label="Copy all prompts"
            className="mb-1"
          />
        )}
        {(conceptReview || imageReview) && (
          <>
            <TextField
              value={draft.overall}
              onChange={(overall) => onDraft({ ...draft, overall })}
            >
              <Label>
                Overall feedback <span className="text-muted">(optional)</span>
              </Label>
              <TextArea rows={3} />
            </TextField>
            {conceptReview && (
              <Checkbox
                isSelected={draft.items.__none === "true"}
                onChange={(v) =>
                  onDraft({
                    ...draft,
                    items: { ...draft.items, __none: v ? "true" : "" },
                  })
                }
              >
                Refine without feedback
              </Checkbox>
            )}
            <Button
              className="accept-button w-full"
              onPress={() =>
                onDecision(isImages ? "accept_images" : "accept_concepts")
              }
              isDisabled={!!pending}
            >
              {pending?.startsWith("accept") && <Spinner size="sm" />}
              <Check size={16} />
              {isImages
                ? "Accept images"
                : generateImages
                  ? "Accept concepts & generate images"
                  : "Finish with these prompts"}
            </Button>
            {conceptReview && generateImages && (
              <Button
                variant="secondary"
                className="w-full"
                onPress={() => onDecision("stop_at_prompts")}
                isDisabled={!!pending}
              >
                {pending?.startsWith("stop") && <Spinner size="sm" />}
                <ClipboardCopy size={16} />
                Stop here, I’ll copy the prompts
              </Button>
            )}
            <Button
              variant="secondary"
              className="w-full"
              onPress={() =>
                onDecision(
                  isImages ? "revise_images" : "revise_concepts",
                  draft.items.__none === "true",
                )
              }
              isDisabled={!!pending}
            >
              {pending?.startsWith("revise") && <Spinner size="sm" />}
              <Send size={16} />
              Request refinement
            </Button>
          </>
        )}
      </Card>
    </div>
  );
}
