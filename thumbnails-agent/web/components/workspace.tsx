"use client";

import { Button, I18nProvider } from "@heroui/react";
import { ChevronRight, Menu, Plus, X } from "lucide-react";
import { useEffect, useState } from "react";
import { usePolling } from "@/hooks/use-polling";
import type { Run } from "@/lib/types";
import { Sidebar } from "./sidebar";
import { NewRunForm } from "./new-run-form";
import { RunDetail } from "./run-detail";

export function Workspace() {
  const runs = usePolling<{ runs: Run[] }>("/api/runs", 1500);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [menuOpen, setMenuOpen] = useState(false);
  useEffect(() => {
    const sync = () =>
      setSelectedId(new URLSearchParams(window.location.search).get("run"));
    sync();
    window.addEventListener("popstate", sync);
    return () => window.removeEventListener("popstate", sync);
  }, []);
  function navigate(id: string | null) {
    setSelectedId(id);
    setMenuOpen(false);
    window.history.pushState(
      null,
      "",
      id ? `/?run=${encodeURIComponent(id)}` : "/",
    );
  }
  return (
    <I18nProvider locale="en-US">
      <div className="workspace grid min-h-dvh md:grid-cols-[16.5rem_minmax(0,1fr)]">
        <a
          className="fixed left-5 top-0 z-50 -translate-y-24 bg-surface px-3 py-2 text-sm text-foreground shadow-overlay transition-transform focus:translate-y-3"
          href="#main-content"
        >
          Skip to content
        </a>
        <Sidebar
          runs={runs.data?.runs || []}
          selectedId={selectedId}
          onSelect={navigate}
          onNew={() => navigate(null)}
          loading={runs.loading}
          error={runs.error}
          onRetry={runs.refresh}
          open={menuOpen}
        />
        <div className="workspace-main min-w-0">
          <header className="topbar sticky top-0 z-30 flex h-16 items-center justify-between border-b border-border bg-background/95 px-4 backdrop-blur sm:h-[4.5rem] sm:px-8">
            <div className="breadcrumbs flex items-center gap-2 text-xs text-muted">
              <Button
                className="mobile-menu md:hidden"
                variant="ghost"
                isIconOnly
                aria-label={
                  menuOpen
                    ? "Close thumbnail history"
                    : "Open thumbnail history"
                }
                aria-expanded={menuOpen}
                aria-controls="run-navigation"
                onPress={() => setMenuOpen(!menuOpen)}
              >
                {menuOpen ? <X size={20} /> : <Menu size={20} />}
              </Button>
              <span className="hidden sm:inline">Workspace</span>
              <ChevronRight className="hidden sm:block" size={14} />
              <strong>
                {selectedId ? "Thumbnail details" : "New thumbnails"}
              </strong>
            </div>
            <div className="topbar-right flex items-center">
              {selectedId ? (
                <Button
                  size="sm"
                  variant="secondary"
                  onPress={() => navigate(null)}
                >
                  <Plus size={15} />
                  New thumbnails
                </Button>
              ) : null}
            </div>
          </header>
          <main id="main-content">
            {selectedId ? (
              <RunDetail
                key={selectedId}
                id={selectedId}
                onChange={runs.refresh}
                onDeleted={() => {
                  navigate(null);
                  runs.refresh();
                }}
              />
            ) : (
              <NewRunForm
                onCreated={(id) => {
                  navigate(id);
                  runs.refresh();
                }}
              />
            )}
          </main>
        </div>
      </div>
    </I18nProvider>
  );
}
