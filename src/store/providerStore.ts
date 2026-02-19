import { create } from "zustand";
import type {
  ProviderConfig,
  ProviderSummary,
  TestResult,
} from "../models/provider";
import {
  providersList,
  providersGet,
  providersUpsert,
  providersDelete,
  secretsSet,
  secretsExists,
  secretsDelete,
  providersTest,
} from "../services/providerCommands";

interface ProviderStore {
  providers: ProviderSummary[];
  selectedProvider: string | null;
  providerDetail: ProviderConfig | null;
  connectionStatus: Record<string, boolean>;
  testResult: TestResult | null;
  testLoading: boolean;

  loadProviders: () => Promise<void>;
  selectProvider: (name: string | null) => Promise<void>;
  upsertProvider: (name: string, config: ProviderConfig) => Promise<void>;
  deleteProvider: (name: string) => Promise<void>;
  connect: (credentialRef: string, secret: string) => Promise<void>;
  disconnect: (credentialRef: string) => Promise<void>;
  checkConnection: (credentialRef: string) => Promise<boolean>;
  testConnection: (providerName: string, profileName: string) => Promise<void>;
  clearTestResult: () => void;
}

export const useProviderStore = create<ProviderStore>((set, get) => ({
  providers: [],
  selectedProvider: null,
  providerDetail: null,
  connectionStatus: {},
  testResult: null,
  testLoading: false,

  loadProviders: async () => {
    try {
      const list = await providersList();
      set({ providers: list });
    } catch {
      set({ providers: [] });
    }
  },

  selectProvider: async (name) => {
    if (!name) {
      set({ selectedProvider: null, providerDetail: null, testResult: null });
      return;
    }
    try {
      const detail = await providersGet(name);
      set({ selectedProvider: name, providerDetail: detail, testResult: null });

      const connStatus: Record<string, boolean> = {};
      for (const [, profile] of Object.entries(detail.profiles)) {
        try {
          connStatus[profile.credentialRef] = await secretsExists(
            profile.credentialRef
          );
        } catch {
          connStatus[profile.credentialRef] = false;
        }
      }
      set((s) => ({
        connectionStatus: { ...s.connectionStatus, ...connStatus },
      }));
    } catch {
      set({ selectedProvider: name, providerDetail: null });
    }
  },

  upsertProvider: async (name, config) => {
    await providersUpsert(name, config);
    await get().loadProviders();
    if (get().selectedProvider === name) {
      await get().selectProvider(name);
    }
  },

  deleteProvider: async (name) => {
    await providersDelete(name);
    if (get().selectedProvider === name) {
      set({ selectedProvider: null, providerDetail: null });
    }
    await get().loadProviders();
  },

  connect: async (credentialRef, secret) => {
    await secretsSet(credentialRef, secret);
    set((s) => ({
      connectionStatus: { ...s.connectionStatus, [credentialRef]: true },
    }));
  },

  disconnect: async (credentialRef) => {
    await secretsDelete(credentialRef);
    set((s) => ({
      connectionStatus: { ...s.connectionStatus, [credentialRef]: false },
    }));
  },

  checkConnection: async (credentialRef) => {
    const exists = await secretsExists(credentialRef);
    set((s) => ({
      connectionStatus: { ...s.connectionStatus, [credentialRef]: exists },
    }));
    return exists;
  },

  testConnection: async (providerName, profileName) => {
    set({ testLoading: true, testResult: null });
    try {
      const result = await providersTest(providerName, profileName);
      set({ testResult: result, testLoading: false });
    } catch (e) {
      set({
        testResult: { ok: false, error: String(e) },
        testLoading: false,
      });
    }
  },

  clearTestResult: () => set({ testResult: null }),
}));
