import { useEffect, useState, useCallback } from "react";
import { useProviderStore } from "../store/providerStore";
import { useProjectStore } from "../store/projectStore";
import { updateGenerationSettings } from "../services/commands";
import type { ProviderConfig, AuthKind, ProfileConfig } from "../models/provider";

function ProjectProviderSelect({
  providers,
}: {
  providers: { name: string; displayName: string; profiles: string[] }[];
}) {
  const projectFile = useProjectStore((s) => s.projectFile);
  const gen = projectFile?.project.settings.generation;
  const [selProvider, setSelProvider] = useState(gen?.videoProvider || "");
  const [selProfile, setSelProfile] = useState(gen?.videoProfile || "");
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    setSelProvider(gen?.videoProvider || "");
    setSelProfile(gen?.videoProfile || "");
  }, [gen?.videoProvider, gen?.videoProfile]);

  const currentProfiles =
    providers.find((p) => p.name === selProvider)?.profiles || [];

  const handleSave = async () => {
    await updateGenerationSettings(
      selProvider || undefined,
      selProfile || undefined
    );
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  };

  if (!projectFile) return null;

  return (
    <div data-testid="project-provider-select" className="space-y-2">
      <span className="text-xs font-medium text-zinc-300">项目默认 Provider / Profile</span>
      <div className="flex gap-2 items-end">
        <label className="block flex-1">
          <span className="text-[10px] text-zinc-400 block mb-1">Provider</span>
          <select
            data-testid="select-default-provider"
            value={selProvider}
            onChange={(e) => {
              setSelProvider(e.target.value);
              setSelProfile("");
            }}
            className="w-full px-2 py-1.5 text-xs bg-zinc-800 border border-zinc-700 rounded text-zinc-200"
          >
            <option value="">— 未选择 —</option>
            {providers.map((p) => (
              <option key={p.name} value={p.name}>
                {p.displayName || p.name}
              </option>
            ))}
          </select>
        </label>
        <label className="block flex-1">
          <span className="text-[10px] text-zinc-400 block mb-1">Profile</span>
          <select
            data-testid="select-default-profile"
            value={selProfile}
            onChange={(e) => setSelProfile(e.target.value)}
            className="w-full px-2 py-1.5 text-xs bg-zinc-800 border border-zinc-700 rounded text-zinc-200"
          >
            <option value="">— 未选择 —</option>
            {currentProfiles.map((p) => (
              <option key={p} value={p}>
                {p}
              </option>
            ))}
          </select>
        </label>
        <button
          data-testid="btn-save-project-provider"
          onClick={handleSave}
          className="px-3 py-1.5 text-xs bg-blue-600 hover:bg-blue-500 text-white rounded"
        >
          {saved ? "已保存" : "保存"}
        </button>
      </div>
    </div>
  );
}

function emptyProvider(): ProviderConfig {
  return {
    displayName: "",
    baseUrl: "",
    auth: { kind: "api_key", header: "Authorization", prefix: "Bearer " },
    test: { method: "GET", path: "/health" },
    profiles: {
      default: {
        model: "",
        timeoutMs: 60000,
        retry: { max: 2, backoffMs: 800 },
        credentialRef: "",
      },
    },
  };
}

export function SettingsPanel({ onClose }: { onClose: () => void }) {
  const {
    providers,
    selectedProvider,
    providerDetail,
    connectionStatus,
    testResult,
    testLoading,
    loadProviders,
    selectProvider,
    upsertProvider,
    deleteProvider,
    connect,
    disconnect,
    testConnection,
    clearTestResult,
  } = useProviderStore();

  const [editName, setEditName] = useState("");
  const [editConfig, setEditConfig] = useState<ProviderConfig>(emptyProvider());
  const [isNew, setIsNew] = useState(false);
  const [secretInputs, setSecretInputs] = useState<Record<string, string>>({});
  const [saveMsg, setSaveMsg] = useState("");
  const [deleteConfirm, setDeleteConfirm] = useState(false);
  const [disconnectConfirm, setDisconnectConfirm] = useState<string | null>(null);
  const [urlError, setUrlError] = useState("");

  useEffect(() => {
    loadProviders();
  }, [loadProviders]);

  useEffect(() => {
    if (providerDetail && selectedProvider) {
      setEditName(selectedProvider);
      setEditConfig(structuredClone(providerDetail));
      setIsNew(false);
    }
  }, [providerDetail, selectedProvider]);

  const handleSelectProvider = useCallback(
    (name: string) => {
      clearTestResult();
      setDeleteConfirm(false);
      setDisconnectConfirm(null);
      setSaveMsg("");
      selectProvider(name);
    },
    [selectProvider, clearTestResult]
  );

  const handleNewProvider = () => {
    setIsNew(true);
    setEditName("");
    setEditConfig(emptyProvider());
    clearTestResult();
    setDeleteConfirm(false);
    setSaveMsg("");
  };

  const validateUrl = (url: string): boolean => {
    try {
      new URL(url);
      setUrlError("");
      return true;
    } catch {
      setUrlError("请输入有效的 URL 格式（如 https://api.example.com）");
      return false;
    }
  };

  const handleSave = async () => {
    if (!editName.trim()) return;
    if (!validateUrl(editConfig.baseUrl)) return;

    for (const [profName, prof] of Object.entries(editConfig.profiles)) {
      if (!profName.trim() || !prof.credentialRef.trim()) {
        setSaveMsg("Profile 名称和 credentialRef 不能为空");
        return;
      }
    }

    try {
      await upsertProvider(editName.trim(), editConfig);
      setIsNew(false);
      setSaveMsg("已保存");
      setTimeout(() => setSaveMsg(""), 2000);
      await selectProvider(editName.trim());
    } catch (e) {
      setSaveMsg(`保存失败: ${e}`);
    }
  };

  const handleDelete = async () => {
    if (!selectedProvider) return;
    if (!deleteConfirm) {
      setDeleteConfirm(true);
      return;
    }
    await deleteProvider(selectedProvider);
    setDeleteConfirm(false);
    setEditName("");
    setEditConfig(emptyProvider());
    setIsNew(false);
  };

  const handleConnect = async (credRef: string) => {
    const secret = secretInputs[credRef];
    if (!secret?.trim()) return;
    await connect(credRef, secret.trim());
    setSecretInputs((s) => ({ ...s, [credRef]: "" }));
  };

  const handleDisconnect = async (credRef: string) => {
    if (disconnectConfirm !== credRef) {
      setDisconnectConfirm(credRef);
      return;
    }
    await disconnect(credRef);
    setDisconnectConfirm(null);
  };

  const handleTest = async (profileName: string) => {
    if (!selectedProvider) return;
    await testConnection(selectedProvider, profileName);
  };

  const setAuthKind = (kind: AuthKind) => {
    setEditConfig((c) => ({
      ...c,
      auth: {
        ...c.auth,
        kind,
        ...(kind === "api_key"
          ? { header: c.auth.header || "Authorization", prefix: c.auth.prefix || "Bearer " }
          : { cookieName: c.auth.cookieName || "sessionid" }),
      },
    }));
  };

  const updateProfile = (profName: string, patch: Partial<ProfileConfig>) => {
    setEditConfig((c) => ({
      ...c,
      profiles: {
        ...c.profiles,
        [profName]: { ...c.profiles[profName], ...patch },
      },
    }));
  };

  const addProfile = () => {
    const name = `profile_${Date.now()}`;
    setEditConfig((c) => ({
      ...c,
      profiles: {
        ...c.profiles,
        [name]: {
          model: "",
          timeoutMs: 60000,
          retry: { max: 2, backoffMs: 800 },
          credentialRef: `cred_${editName || "new"}_${name}`,
        },
      },
    }));
  };

  const removeProfile = (profName: string) => {
    setEditConfig((c) => {
      const next = { ...c.profiles };
      delete next[profName];
      return { ...c, profiles: next };
    });
  };

  return (
    <div
      data-testid="settings-panel"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60"
      onClick={(e) => { if (e.target === e.currentTarget) onClose(); }}
    >
      <div className="bg-zinc-900 border border-zinc-700 rounded-lg shadow-2xl w-[900px] max-h-[80vh] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-zinc-700">
          <h2 className="text-lg font-semibold">设置 — Providers</h2>
          <button
            data-testid="btn-close-settings"
            onClick={onClose}
            className="text-zinc-400 hover:text-zinc-200 text-xl leading-none"
          >
            ✕
          </button>
        </div>

        <div className="flex flex-1 overflow-hidden min-h-0">
          {/* Left: Provider list */}
          <div className="w-56 border-r border-zinc-700 flex flex-col overflow-y-auto">
            <div className="p-3 border-b border-zinc-800">
              <button
                data-testid="btn-add-provider"
                onClick={handleNewProvider}
                className="w-full px-3 py-1.5 text-xs bg-blue-600 hover:bg-blue-500 text-white rounded"
              >
                + 新增 Provider
              </button>
            </div>
            <div data-testid="provider-list" className="flex-1 overflow-y-auto">
              {providers.map((p) => (
                <button
                  key={p.name}
                  data-testid={`provider-item-${p.name}`}
                  onClick={() => handleSelectProvider(p.name)}
                  className={`w-full text-left px-4 py-2.5 text-xs border-b border-zinc-800 transition-colors ${
                    selectedProvider === p.name
                      ? "bg-zinc-800 text-zinc-100"
                      : "text-zinc-400 hover:bg-zinc-800/50 hover:text-zinc-200"
                  }`}
                >
                  <div className="font-medium">{p.displayName || p.name}</div>
                  <div className="text-[10px] text-zinc-500 mt-0.5">
                    {p.authKind === "api_key" ? "API Key" : "Session Cookie"} · {p.profiles.length} profile(s)
                  </div>
                </button>
              ))}
              {providers.length === 0 && (
                <div className="px-4 py-6 text-xs text-zinc-500 text-center">
                  暂无 Provider
                </div>
              )}
            </div>
          </div>

          {/* Right: Detail */}
          <div className="flex-1 overflow-y-auto p-6">
            {/* Project default provider/profile */}
            <ProjectProviderSelect providers={providers} />

            <hr className="border-zinc-700 my-4" />

            {!selectedProvider && !isNew ? (
              <div className="text-zinc-500 text-sm text-center mt-12">
                请从左侧选择或新增一个 Provider
              </div>
            ) : (
              <div className="space-y-4">
                {/* Basic info */}
                <div className="grid grid-cols-2 gap-3">
                  <label className="block">
                    <span className="text-[10px] text-zinc-400 block mb-1">名称 (ID)</span>
                    <input
                      data-testid="input-provider-name"
                      value={editName}
                      onChange={(e) => setEditName(e.target.value)}
                      disabled={!isNew}
                      className="w-full px-2 py-1.5 text-xs bg-zinc-800 border border-zinc-700 rounded text-zinc-200 disabled:opacity-50"
                      placeholder="如 fooVideo"
                    />
                  </label>
                  <label className="block">
                    <span className="text-[10px] text-zinc-400 block mb-1">显示名称</span>
                    <input
                      data-testid="input-display-name"
                      value={editConfig.displayName}
                      onChange={(e) => setEditConfig((c) => ({ ...c, displayName: e.target.value }))}
                      className="w-full px-2 py-1.5 text-xs bg-zinc-800 border border-zinc-700 rounded text-zinc-200"
                      placeholder="如 Foo Video"
                    />
                  </label>
                </div>

                <label className="block">
                  <span className="text-[10px] text-zinc-400 block mb-1">Base URL</span>
                  <input
                    data-testid="input-base-url"
                    value={editConfig.baseUrl}
                    onChange={(e) => {
                      setEditConfig((c) => ({ ...c, baseUrl: e.target.value }));
                      if (urlError) validateUrl(e.target.value);
                    }}
                    className={`w-full px-2 py-1.5 text-xs bg-zinc-800 border rounded text-zinc-200 ${
                      urlError ? "border-red-500" : "border-zinc-700"
                    }`}
                    placeholder="https://api.example.com"
                  />
                  {urlError && <span className="text-[10px] text-red-400 mt-0.5">{urlError}</span>}
                </label>

                {/* Auth section */}
                <div className="space-y-2">
                  <span className="text-[10px] text-zinc-400 block">认证类型</span>
                  <div className="flex gap-2">
                    <button
                      data-testid="auth-kind-api-key"
                      onClick={() => setAuthKind("api_key")}
                      className={`px-3 py-1 text-xs rounded ${
                        editConfig.auth.kind === "api_key"
                          ? "bg-blue-600 text-white"
                          : "bg-zinc-800 text-zinc-400 hover:bg-zinc-700"
                      }`}
                    >
                      API Key
                    </button>
                    <button
                      data-testid="auth-kind-session-cookie"
                      onClick={() => setAuthKind("session_cookie")}
                      className={`px-3 py-1 text-xs rounded ${
                        editConfig.auth.kind === "session_cookie"
                          ? "bg-blue-600 text-white"
                          : "bg-zinc-800 text-zinc-400 hover:bg-zinc-700"
                      }`}
                    >
                      Session Cookie
                    </button>
                  </div>

                  {editConfig.auth.kind === "api_key" ? (
                    <div className="grid grid-cols-2 gap-3">
                      <label className="block">
                        <span className="text-[10px] text-zinc-400 block mb-1">Header</span>
                        <input
                          data-testid="input-auth-header"
                          value={editConfig.auth.header || ""}
                          onChange={(e) =>
                            setEditConfig((c) => ({
                              ...c,
                              auth: { ...c.auth, header: e.target.value },
                            }))
                          }
                          className="w-full px-2 py-1.5 text-xs bg-zinc-800 border border-zinc-700 rounded text-zinc-200"
                          placeholder="Authorization"
                        />
                      </label>
                      <label className="block">
                        <span className="text-[10px] text-zinc-400 block mb-1">Prefix</span>
                        <input
                          data-testid="input-auth-prefix"
                          value={editConfig.auth.prefix || ""}
                          onChange={(e) =>
                            setEditConfig((c) => ({
                              ...c,
                              auth: { ...c.auth, prefix: e.target.value },
                            }))
                          }
                          className="w-full px-2 py-1.5 text-xs bg-zinc-800 border border-zinc-700 rounded text-zinc-200"
                          placeholder="Bearer "
                        />
                      </label>
                    </div>
                  ) : (
                    <label className="block">
                      <span className="text-[10px] text-zinc-400 block mb-1">Cookie Name</span>
                      <input
                        data-testid="input-cookie-name"
                        value={editConfig.auth.cookieName || ""}
                        onChange={(e) =>
                          setEditConfig((c) => ({
                            ...c,
                            auth: { ...c.auth, cookieName: e.target.value },
                          }))
                        }
                        className="w-full px-2 py-1.5 text-xs bg-zinc-800 border border-zinc-700 rounded text-zinc-200"
                        placeholder="sessionid"
                      />
                    </label>
                  )}
                </div>

                {/* Test endpoint */}
                <div className="grid grid-cols-2 gap-3">
                  <label className="block">
                    <span className="text-[10px] text-zinc-400 block mb-1">Test Method</span>
                    <select
                      data-testid="select-test-method"
                      value={editConfig.test?.method || "GET"}
                      onChange={(e) =>
                        setEditConfig((c) => ({
                          ...c,
                          test: { method: e.target.value, path: c.test?.path || "/health" },
                        }))
                      }
                      className="w-full px-2 py-1.5 text-xs bg-zinc-800 border border-zinc-700 rounded text-zinc-200"
                    >
                      <option value="GET">GET</option>
                      <option value="POST">POST</option>
                      <option value="HEAD">HEAD</option>
                    </select>
                  </label>
                  <label className="block">
                    <span className="text-[10px] text-zinc-400 block mb-1">Test Path</span>
                    <input
                      data-testid="input-test-path"
                      value={editConfig.test?.path || ""}
                      onChange={(e) =>
                        setEditConfig((c) => ({
                          ...c,
                          test: { method: c.test?.method || "GET", path: e.target.value },
                        }))
                      }
                      className="w-full px-2 py-1.5 text-xs bg-zinc-800 border border-zinc-700 rounded text-zinc-200"
                      placeholder="/health"
                    />
                  </label>
                </div>

                {/* Profiles */}
                <div className="space-y-3">
                  <div className="flex items-center justify-between">
                    <span className="text-xs font-medium text-zinc-300">Profiles</span>
                    <button
                      data-testid="btn-add-profile"
                      onClick={addProfile}
                      className="px-2 py-0.5 text-[10px] bg-zinc-700 hover:bg-zinc-600 text-zinc-300 rounded"
                    >
                      + 添加 Profile
                    </button>
                  </div>

                  {Object.entries(editConfig.profiles).map(([profName, prof]) => (
                    <div
                      key={profName}
                      data-testid={`profile-card-${profName}`}
                      className="p-3 bg-zinc-800/50 border border-zinc-700 rounded space-y-2"
                    >
                      <div className="flex items-center justify-between">
                        <span className="text-xs font-medium text-zinc-200">{profName}</span>
                        <div className="flex items-center gap-2">
                          <span
                            data-testid={`conn-status-${prof.credentialRef}`}
                            className={`inline-block w-2 h-2 rounded-full ${
                              connectionStatus[prof.credentialRef] ? "bg-green-500" : "bg-zinc-500"
                            }`}
                          />
                          <span className="text-[10px] text-zinc-400">
                            {connectionStatus[prof.credentialRef] ? "Connected" : "Not connected"}
                          </span>
                          {Object.keys(editConfig.profiles).length > 1 && (
                            <button
                              onClick={() => removeProfile(profName)}
                              className="text-[10px] text-red-400 hover:text-red-300"
                            >
                              删除
                            </button>
                          )}
                        </div>
                      </div>

                      <div className="grid grid-cols-2 gap-2">
                        <label className="block">
                          <span className="text-[10px] text-zinc-500 block mb-0.5">Model</span>
                          <input
                            value={prof.model}
                            onChange={(e) => updateProfile(profName, { model: e.target.value })}
                            className="w-full px-2 py-1 text-xs bg-zinc-800 border border-zinc-700 rounded text-zinc-200"
                            placeholder="video-v1"
                          />
                        </label>
                        <label className="block">
                          <span className="text-[10px] text-zinc-500 block mb-0.5">credentialRef</span>
                          <input
                            data-testid={`input-cred-ref-${profName}`}
                            value={prof.credentialRef}
                            onChange={(e) =>
                              updateProfile(profName, { credentialRef: e.target.value })
                            }
                            className="w-full px-2 py-1 text-xs bg-zinc-800 border border-zinc-700 rounded text-zinc-200"
                            placeholder="cred_foo_default"
                          />
                        </label>
                        <label className="block">
                          <span className="text-[10px] text-zinc-500 block mb-0.5">Timeout (ms)</span>
                          <input
                            type="number"
                            value={prof.timeoutMs}
                            onChange={(e) =>
                              updateProfile(profName, { timeoutMs: Number(e.target.value) || 60000 })
                            }
                            className="w-full px-2 py-1 text-xs bg-zinc-800 border border-zinc-700 rounded text-zinc-200"
                          />
                        </label>
                        <div className="grid grid-cols-2 gap-1">
                          <label className="block">
                            <span className="text-[10px] text-zinc-500 block mb-0.5">Retry Max</span>
                            <input
                              type="number"
                              value={prof.retry.max}
                              onChange={(e) =>
                                updateProfile(profName, {
                                  retry: { ...prof.retry, max: Number(e.target.value) || 0 },
                                })
                              }
                              className="w-full px-2 py-1 text-xs bg-zinc-800 border border-zinc-700 rounded text-zinc-200"
                            />
                          </label>
                          <label className="block">
                            <span className="text-[10px] text-zinc-500 block mb-0.5">Backoff (ms)</span>
                            <input
                              type="number"
                              value={prof.retry.backoffMs}
                              onChange={(e) =>
                                updateProfile(profName, {
                                  retry: { ...prof.retry, backoffMs: Number(e.target.value) || 0 },
                                })
                              }
                              className="w-full px-2 py-1 text-xs bg-zinc-800 border border-zinc-700 rounded text-zinc-200"
                            />
                          </label>
                        </div>
                      </div>

                      {/* Connect / Disconnect / Test */}
                      <div className="flex items-center gap-2 pt-1">
                        {!connectionStatus[prof.credentialRef] ? (
                          <div className="flex items-center gap-1 flex-1">
                            <input
                              data-testid={`input-secret-${prof.credentialRef}`}
                              type="password"
                              value={secretInputs[prof.credentialRef] || ""}
                              onChange={(e) =>
                                setSecretInputs((s) => ({ ...s, [prof.credentialRef]: e.target.value }))
                              }
                              placeholder={
                                editConfig.auth.kind === "api_key" ? "输入 API Key..." : "输入 Session ID..."
                              }
                              className="flex-1 px-2 py-1 text-xs bg-zinc-800 border border-zinc-700 rounded text-zinc-200"
                            />
                            <button
                              data-testid={`btn-connect-${prof.credentialRef}`}
                              onClick={() => handleConnect(prof.credentialRef)}
                              className="px-2 py-1 text-xs bg-green-700 hover:bg-green-600 text-white rounded"
                            >
                              Connect
                            </button>
                          </div>
                        ) : (
                          <button
                            data-testid={`btn-disconnect-${prof.credentialRef}`}
                            onClick={() => handleDisconnect(prof.credentialRef)}
                            className={`px-2 py-1 text-xs rounded ${
                              disconnectConfirm === prof.credentialRef
                                ? "bg-red-600 hover:bg-red-500 text-white"
                                : "bg-zinc-700 hover:bg-zinc-600 text-zinc-300"
                            }`}
                          >
                            {disconnectConfirm === prof.credentialRef ? "确认 Disconnect" : "Disconnect"}
                          </button>
                        )}
                        <button
                          data-testid={`btn-test-${profName}`}
                          onClick={() => handleTest(profName)}
                          disabled={testLoading}
                          className="px-2 py-1 text-xs bg-zinc-700 hover:bg-zinc-600 text-zinc-300 rounded disabled:opacity-50"
                        >
                          {testLoading ? "测试中..." : "Test"}
                        </button>
                      </div>
                    </div>
                  ))}
                </div>

                {/* Test result */}
                {testResult && (
                  <div
                    data-testid="test-result"
                    className={`p-2 text-xs rounded ${
                      testResult.ok
                        ? "bg-green-900/30 border border-green-700 text-green-300"
                        : "bg-red-900/30 border border-red-700 text-red-300"
                    }`}
                  >
                    {testResult.ok
                      ? `✓ 连接成功 (${testResult.latencyMs}ms)`
                      : `✗ ${testResult.error || "连接失败"}`}
                  </div>
                )}

                {/* Save / Delete */}
                <div className="flex items-center gap-2 pt-2 border-t border-zinc-700">
                  <button
                    data-testid="btn-save-provider"
                    onClick={handleSave}
                    className="px-4 py-1.5 text-xs bg-blue-600 hover:bg-blue-500 text-white rounded"
                  >
                    保存 Provider
                  </button>
                  {!isNew && selectedProvider && (
                    <button
                      data-testid="btn-delete-provider"
                      onClick={handleDelete}
                      className={`px-4 py-1.5 text-xs rounded ${
                        deleteConfirm
                          ? "bg-red-600 hover:bg-red-500 text-white"
                          : "bg-zinc-700 hover:bg-zinc-600 text-zinc-300"
                      }`}
                    >
                      {deleteConfirm ? "确认删除" : "删除 Provider"}
                    </button>
                  )}
                  {saveMsg && (
                    <span className="text-xs text-zinc-400 ml-2">{saveMsg}</span>
                  )}
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
