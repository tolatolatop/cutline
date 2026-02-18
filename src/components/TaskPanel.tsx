import { useState } from "react";
import { useProjectStore } from "../store/projectStore";
import * as commands from "../services/commands";
import type { Task, TaskEvent } from "../models/project";

function stateColor(state: string): string {
  switch (state) {
    case "queued":
      return "bg-zinc-600 text-zinc-200";
    case "running":
      return "bg-blue-600 text-white";
    case "succeeded":
      return "bg-green-700 text-green-100";
    case "failed":
      return "bg-red-700 text-red-100";
    case "canceled":
      return "bg-yellow-700 text-yellow-100";
    default:
      return "bg-zinc-600 text-zinc-300";
  }
}

function stateLabel(state: string): string {
  switch (state) {
    case "queued": return "排队中";
    case "running": return "执行中";
    case "succeeded": return "已完成";
    case "failed": return "失败";
    case "canceled": return "已取消";
    default: return state;
  }
}

function kindLabel(kind: string): string {
  switch (kind) {
    case "probe": return "媒体探测";
    case "thumb": return "缩略图";
    case "proxy": return "代理视频";
    case "generate": return "AI 生成";
    case "export": return "导出";
    default: return kind;
  }
}

function ProgressBar({ percent }: { percent: number }) {
  return (
    <div className="w-full bg-zinc-700 rounded-full h-1.5 mt-1">
      <div
        className="bg-blue-500 h-1.5 rounded-full transition-all duration-300"
        style={{ width: `${Math.min(100, Math.max(0, percent))}%` }}
      />
    </div>
  );
}

function EventLog({ events }: { events: TaskEvent[] }) {
  const recent = events.slice(-20);
  return (
    <div className="mt-2 max-h-32 overflow-y-auto bg-zinc-900 rounded p-2 text-[11px] font-mono space-y-0.5">
      {recent.map((ev, i) => (
        <div key={i} className="flex gap-2">
          <span className="text-zinc-600 shrink-0">
            {new Date(ev.t).toLocaleTimeString()}
          </span>
          <span
            className={
              ev.level === "error"
                ? "text-red-400"
                : ev.level === "warn"
                ? "text-yellow-400"
                : "text-zinc-400"
            }
          >
            [{ev.level}]
          </span>
          <span className="text-zinc-300 break-all">{ev.msg}</span>
        </div>
      ))}
      {events.length === 0 && (
        <div className="text-zinc-600">暂无日志</div>
      )}
    </div>
  );
}

function TaskRow({ task }: { task: Task }) {
  const [expanded, setExpanded] = useState(false);
  const [actionLoading, setActionLoading] = useState(false);

  const handleRetry = async (e: React.MouseEvent) => {
    e.stopPropagation();
    setActionLoading(true);
    try {
      await commands.taskRetry(task.taskId);
    } catch (err) {
      console.error("retry failed:", err);
    }
    setActionLoading(false);
  };

  const handleCancel = async (e: React.MouseEvent) => {
    e.stopPropagation();
    setActionLoading(true);
    try {
      await commands.taskCancel(task.taskId);
    } catch (err) {
      console.error("cancel failed:", err);
    }
    setActionLoading(false);
  };

  return (
    <div className="border-b border-zinc-800">
      <div
        onClick={() => setExpanded(!expanded)}
        className="flex items-center gap-2 px-3 py-2 cursor-pointer hover:bg-zinc-800/50"
      >
        <span className="text-zinc-500 text-xs">{expanded ? "▼" : "▶"}</span>
        <span className="text-xs text-zinc-300 font-medium">
          {kindLabel(task.kind)}
        </span>
        <span
          className={`px-1.5 py-0.5 text-[10px] rounded ${stateColor(task.state)}`}
        >
          {stateLabel(task.state)}
        </span>
        {task.retries.count > 0 && (
          <span className="text-[10px] text-zinc-500">
            (重试 #{task.retries.count})
          </span>
        )}
        <span className="ml-auto flex gap-1">
          {(task.state === "failed" || task.state === "canceled") && (
            <button
              onClick={handleRetry}
              disabled={actionLoading}
              className="px-1.5 py-0.5 text-[10px] bg-blue-700 hover:bg-blue-600 text-white rounded disabled:opacity-50"
            >
              重试
            </button>
          )}
          {(task.state === "queued" || task.state === "running") && (
            <button
              onClick={handleCancel}
              disabled={actionLoading}
              className="px-1.5 py-0.5 text-[10px] bg-zinc-600 hover:bg-zinc-500 text-zinc-200 rounded disabled:opacity-50"
            >
              取消
            </button>
          )}
        </span>
      </div>

      {task.state === "running" && task.progress?.percent != null && (
        <div className="px-3 pb-1">
          <ProgressBar percent={task.progress.percent} />
          {task.progress.message && (
            <div className="text-[10px] text-zinc-500 mt-0.5">
              {task.progress.phase}: {task.progress.message}
            </div>
          )}
        </div>
      )}

      {expanded && (
        <div className="px-3 pb-2">
          {task.error && (
            <div className="mt-1 p-2 bg-red-900/30 rounded text-xs">
              <div className="text-red-300 font-medium">
                {task.error.code}: {task.error.message}
              </div>
              {task.error.detail && (
                <pre className="text-red-400/70 text-[10px] mt-1 whitespace-pre-wrap break-all max-h-24 overflow-y-auto">
                  {task.error.detail}
                </pre>
              )}
            </div>
          )}
          <div className="mt-1 text-[10px] text-zinc-600 space-y-0.5">
            <div>ID: {task.taskId}</div>
            <div>创建: {new Date(task.createdAt).toLocaleString()}</div>
            <div>更新: {new Date(task.updatedAt).toLocaleString()}</div>
            {task.deps.length > 0 && <div>依赖: {task.deps.join(", ")}</div>}
          </div>
          <EventLog events={task.events} />
        </div>
      )}
    </div>
  );
}

export function TaskPanel() {
  const { projectFile } = useProjectStore();
  const [filter, setFilter] = useState<string>("all");

  if (!projectFile) return null;

  const tasks = projectFile.tasks;
  const filtered =
    filter === "all"
      ? tasks
      : tasks.filter((t) => t.state === filter);

  const counts = {
    all: tasks.length,
    queued: tasks.filter((t) => t.state === "queued").length,
    running: tasks.filter((t) => t.state === "running").length,
    failed: tasks.filter((t) => t.state === "failed").length,
    succeeded: tasks.filter((t) => t.state === "succeeded").length,
  };

  return (
    <div className="flex flex-col h-full">
      <div className="px-3 py-2 text-xs text-zinc-400 font-semibold border-b border-zinc-800 flex items-center gap-2">
        <span>任务 ({tasks.length})</span>
        {counts.running > 0 && (
          <span className="px-1.5 py-0.5 text-[10px] bg-blue-600 text-white rounded">
            {counts.running} 执行中
          </span>
        )}
        {counts.failed > 0 && (
          <span className="px-1.5 py-0.5 text-[10px] bg-red-700 text-red-100 rounded">
            {counts.failed} 失败
          </span>
        )}
      </div>

      <div className="px-3 py-1 flex gap-1 border-b border-zinc-800">
        {(["all", "running", "queued", "failed", "succeeded"] as const).map(
          (f) => (
            <button
              key={f}
              onClick={() => setFilter(f)}
              className={`px-2 py-0.5 text-[10px] rounded ${
                filter === f
                  ? "bg-zinc-600 text-zinc-100"
                  : "text-zinc-500 hover:text-zinc-300"
              }`}
            >
              {f === "all" ? "全部" : stateLabel(f)} ({counts[f] ?? 0})
            </button>
          )
        )}
      </div>

      <div className="flex-1 overflow-y-auto">
        {filtered.length === 0 ? (
          <div className="flex items-center justify-center h-20 text-zinc-600 text-xs">
            暂无任务
          </div>
        ) : (
          [...filtered].reverse().map((task) => (
            <TaskRow key={task.taskId} task={task} />
          ))
        )}
      </div>
    </div>
  );
}
