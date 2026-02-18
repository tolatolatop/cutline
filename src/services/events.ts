import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { Task } from "../models/project";

export function subscribeTaskUpdates(
  onUpdate: (task: Task) => void
): Promise<UnlistenFn> {
  return listen<{ task: Task }>("task:updated", (event) => {
    onUpdate(event.payload.task);
  });
}

export function subscribeProjectUpdates(
  onUpdate: () => void
): Promise<UnlistenFn> {
  return listen("project:updated", () => {
    onUpdate();
  });
}
