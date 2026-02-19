import { invoke } from "@tauri-apps/api/core";
import type { ProviderConfig, ProviderSummary, TestResult } from "../models/provider";

export async function providersList(): Promise<ProviderSummary[]> {
  return invoke<ProviderSummary[]>("providers_list");
}

export async function providersGet(name: string): Promise<ProviderConfig> {
  return invoke<ProviderConfig>("providers_get", { name });
}

export async function providersUpsert(
  name: string,
  config: ProviderConfig
): Promise<void> {
  return invoke("providers_upsert", { name, config });
}

export async function providersDelete(name: string): Promise<void> {
  return invoke("providers_delete", { name });
}

export async function secretsSet(
  credentialRef: string,
  secret: string
): Promise<void> {
  return invoke("secrets_set", { credentialRef, secret });
}

export async function secretsExists(credentialRef: string): Promise<boolean> {
  return invoke<boolean>("secrets_exists", { credentialRef });
}

export async function secretsDelete(credentialRef: string): Promise<void> {
  return invoke("secrets_delete", { credentialRef });
}

export async function providersTest(
  providerName: string,
  profileName: string
): Promise<TestResult> {
  return invoke<TestResult>("providers_test", { providerName, profileName });
}
