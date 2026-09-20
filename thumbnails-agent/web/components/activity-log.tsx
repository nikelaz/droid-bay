"use client";

import { Button, Card } from "@heroui/react";
import { CheckCheck, ChevronDown, MessageSquare, Terminal } from "lucide-react";
import { useState } from "react";
import type { Message } from "@/lib/types";
import { timeLabel } from "@/lib/format";

const roles: Record<string, string> = {
  event: "Pipeline update",
  prompt: "Agent instructions",
  assistant: "Agent response",
  user: "Your feedback",
};
export function ActivityLog({ messages }: { messages: Message[] }) {
  const [all, setAll] = useState(false);
  const visible = all
    ? messages
    : messages.filter((message) => ["event", "user"].includes(message.role));
  return (
    <section className="activity-section max-w-4xl">
      <div className="content-heading mb-6 flex items-center justify-between gap-4">
        <div>
          <h2 className="font-display text-lg font-semibold">
            Behind the thumbnails
          </h2>
          <p className="mt-1 text-xs leading-5 text-muted">
            A record of every step, from first prompt to final review.
          </p>
        </div>
        <Button size="sm" variant="secondary" onPress={() => setAll(!all)}>
          {all ? "Show updates only" : "Show full log"}
        </Button>
      </div>
      {visible.length === 0 && (
        <Card className="panel empty-panel p-8 text-center text-muted">
          Waiting for the first update…
        </Card>
      )}
      <ol className="activity-list m-0 list-none p-0">
        {visible.map((message) => (
          <li
            key={message.id}
            className={`activity-item activity-${message.role} flex gap-4 pb-7`}
          >
            <span className="activity-icon grid size-8 shrink-0 place-items-center rounded-full border border-border bg-accent-soft text-muted">
              {message.role === "event" ? (
                <CheckCheck size={16} />
              ) : message.role === "user" ? (
                <MessageSquare size={16} />
              ) : (
                <Terminal size={16} />
              )}
            </span>
            <div className="activity-body min-w-0 flex-1 border-b border-border pb-6 pt-1">
              <div className="activity-meta flex flex-wrap items-center gap-3 text-xs text-muted">
                <strong className="text-sm font-medium text-foreground">
                  {roles[message.role] || message.role}
                </strong>
                <span>{message.phase.replaceAll("_", " ")}</span>
                <time className="sm:ml-auto">
                  {timeLabel(message.created_at)}
                </time>
              </div>
              {["prompt", "assistant"].includes(message.role) ? (
                <details>
                  <summary className="flex items-center gap-2 text-xs text-muted">
                    View{" "}
                    {message.role === "prompt" ? "instructions" : "response"}
                    <ChevronDown size={14} />
                  </summary>
                  <pre className="mt-3 max-h-[30rem] overflow-auto whitespace-pre-wrap break-words rounded-lg border border-border bg-background-secondary p-4 text-xs leading-6 text-muted">
                    {message.content}
                  </pre>
                </details>
              ) : (
                <p className="mt-3 whitespace-pre-wrap text-sm leading-7 text-muted">
                  {message.content}
                </p>
              )}
            </div>
          </li>
        ))}
      </ol>
    </section>
  );
}
