import { statusLabels } from "@/lib/format";
export function StatusBadge({ status }: { status: string }) {
  const tone =
    status === "review"
      ? "bg-warning-soft text-warning-soft-foreground"
      : status === "accepted"
        ? "bg-success-soft text-success-soft-foreground"
        : status === "error"
          ? "bg-danger-soft text-danger-soft-foreground"
          : ["running_research", "running_critique", "running_fix"].includes(
                status,
              )
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
