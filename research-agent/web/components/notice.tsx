import { AlertCircle, Info } from "lucide-react";
export function Notice({
  children,
  warning = false,
}: {
  children: React.ReactNode;
  warning?: boolean;
}) {
  const Icon = warning ? Info : AlertCircle;
  return (
    <div
      className={`notice mb-4 flex items-start gap-2 rounded-lg border px-4 py-3 text-sm leading-6 ${warning ? "notice-warning border-warning/30 bg-warning-soft text-warning-soft-foreground" : "notice-error border-danger/30 bg-danger-soft text-danger-soft-foreground"}`}
      role={warning ? "status" : "alert"}
    >
      <Icon className="mt-1" size={17} />
      <div>{children}</div>
    </div>
  );
}
