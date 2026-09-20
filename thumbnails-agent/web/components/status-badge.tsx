import { statusLabels } from "@/lib/format";
export function StatusBadge({ status }: { status: string }) {
  const tone = ["concept_review", "image_review"].includes(status)
    ? "bg-warning-soft text-warning-soft-foreground"
    : ["accepted", "prompts_ready"].includes(status)
      ? "bg-success-soft text-success-soft-foreground"
      : status === "error"
        ? "bg-danger-soft text-danger-soft-foreground"
        : [
              "running_concepts",
              "running_critique",
              "running_refine",
              "running_images",
            ].includes(status)
          ? "bg-accent-soft text-accent-soft-foreground"
          : "bg-default text-muted";
  return (
    <span
      className={`status-badge inline-flex items-center gap-1.5 whitespace-nowrap rounded-md px-2 py-1 text-xs font-medium ${tone}`}
    >
      <span className="status-dot size-1.5 rounded-full bg-current" />
      {statusLabels[status] || status}
    </span>
  );
}
