import { invoke } from "@tauri-apps/api/core";
import type {
  JimengGenerateResult,
  JimengCreditInfo,
} from "./types";

export async function jimengGenerateImage(
  providerName: string,
  profileName: string,
  prompt: string,
  model?: string,
  ratio?: string,
  negativePrompt?: string,
  imageCount?: number,
): Promise<JimengGenerateResult> {
  return invoke<JimengGenerateResult>("jimeng_generate_image", {
    providerName,
    profileName,
    prompt,
    model,
    ratio,
    negativePrompt,
    imageCount,
  });
}

export async function jimengTaskStatus(
  providerName: string,
  profileName: string,
  historyIds: string[],
): Promise<unknown> {
  return invoke("jimeng_task_status", {
    providerName,
    profileName,
    historyIds,
  });
}

export async function jimengCreditBalance(
  providerName: string,
  profileName: string,
): Promise<JimengCreditInfo> {
  return invoke<JimengCreditInfo>("jimeng_credit_balance", {
    providerName,
    profileName,
  });
}
