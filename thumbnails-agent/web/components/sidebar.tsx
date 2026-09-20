"use client";

import { Button, Input } from "@heroui/react";
import { ImageIcon, ChevronRight, Plus } from "lucide-react";
import { useState } from "react";
import type { Run } from "@/lib/types";
import { dateLabel } from "@/lib/format";
import { StatusBadge } from "./status-badge";

export function Sidebar({
  runs,
  selectedId,
  onSelect,
  onNew,
  loading,
  error,
  onRetry,
  open,
}: {
  runs: Run[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  onNew: () => void;
  loading: boolean;
  error?: Error;
  onRetry: () => void;
  open: boolean;
}) {
  const [search, setSearch] = useState("");
  const visible = runs.filter((run) =>
    run.topic.toLowerCase().includes(search.toLowerCase()),
  );
  return (
    <aside
      className={`sidebar ${open ? "flex" : "hidden"} fixed inset-x-0 top-16 z-40 h-[calc(100dvh-4rem)] flex-col overflow-hidden border-r border-border bg-background-secondary px-4 py-6 shadow-overlay md:sticky md:top-0 md:flex md:h-dvh md:w-auto md:shadow-none`}
      id="run-navigation"
    >
      <a
        href="/"
        className="brand flex items-center gap-3 font-display text-base font-bold no-underline"
        onClick={(event) => {
          event.preventDefault();
          onNew();
        }}
      >
        <span className="brand-symbol grid size-10 place-items-center rounded-lg bg-accent text-accent-foreground">
          <ImageIcon size={22} />
        </span>
        <span>
          Thumbnail Agent
          <small className="mt-1 block text-xs font-normal text-muted">
            Your thumbnail workspace
          </small>
        </span>
      </a>
      <Button
        className="new-run-button mt-8 w-full justify-start"
        onPress={onNew}
      >
        <Plus size={17} />
        New thumbnails
      </Button>
      <div className="sidebar-heading mt-7 mb-3 flex items-center justify-between text-xs font-medium uppercase tracking-wider text-muted">
        <span>YOUR THUMBNAILS</span>
        <span className="count rounded-md bg-default px-2 py-0.5 text-xs text-muted">
          {runs.length}
        </span>
      </div>
      <Input
        className="mb-4"
        fullWidth
        aria-label="Search thumbnails"
        placeholder="Find a thumbnail run…"
        value={search}
        onChange={(event) => setSearch(event.target.value)}
      />
      <nav
        aria-label="Thumbnail history"
        className="run-list min-h-0 flex-1 overflow-y-auto -mx-2 px-2"
      >
        {error && (
          <div
            className="sidebar-empty flex flex-col items-center gap-3 px-2 py-7 text-center text-sm text-muted"
            role="alert"
          >
            Couldn’t refresh your thumbnails.
            <Button size="sm" variant="tertiary" onPress={onRetry}>
              Retry
            </Button>
          </div>
        )}
        {loading && runs.length === 0 && !error && (
          <p className="sidebar-empty px-2 py-7 text-center text-sm text-muted">
            Loading your thumbnails…
          </p>
        )}
        {!loading && !error && visible.length === 0 && (
          <div className="sidebar-empty flex flex-col items-center gap-3 px-2 py-7 text-center text-sm text-muted">
            <ImageIcon size={23} />
            <p className="max-w-44">
              {search
                ? "No matching thumbnails."
                : "Your next video starts with a striking thumbnail."}
            </p>
            {!search && (
              <span className="text-xs">
                Your thumbnail runs will appear here.
              </span>
            )}
          </div>
        )}
        {visible.map((run) => (
          <button
            key={run.id}
            className={`run-item mb-1 block w-full rounded-lg border px-3 py-3 text-left transition-colors ${selectedId === run.id ? "border-border bg-background-tertiary" : "border-transparent hover:bg-background-tertiary"}`}
            onClick={() => onSelect(run.id)}
            aria-current={selectedId === run.id ? "page" : undefined}
          >
            <span className="run-item-title flex items-start justify-between gap-2 text-sm font-medium leading-6 break-words">
              {run.topic}
              <ChevronRight className="mt-1 text-muted" size={14} />
            </span>
            <span className="run-item-meta mt-2 flex items-center justify-between gap-2">
              <StatusBadge status={run.status} />
              <time className="whitespace-nowrap text-xs text-muted">
                {dateLabel(run.created_at)}
              </time>
            </span>
          </button>
        ))}
      </nav>
    </aside>
  );
}
