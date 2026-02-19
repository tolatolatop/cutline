export interface ProvidersFile {
  version: number;
  providers: Record<string, ProviderConfig>;
}

export interface ProviderConfig {
  displayName: string;
  baseUrl: string;
  auth: AuthConfig;
  test?: TestEndpoint;
  profiles: Record<string, ProfileConfig>;
}

export type AuthKind = "api_key" | "session_cookie";

export interface AuthConfig {
  kind: AuthKind;
  header?: string;
  prefix?: string;
  cookieName?: string;
}

export interface TestEndpoint {
  method: string;
  path: string;
}

export interface ProfileConfig {
  model: string;
  timeoutMs: number;
  retry: RetryConfig;
  credentialRef: string;
}

export interface RetryConfig {
  max: number;
  backoffMs: number;
}

export interface ProviderSummary {
  name: string;
  displayName: string;
  authKind: AuthKind;
  profiles: string[];
}

export interface TestResult {
  ok: boolean;
  latencyMs?: number;
  error?: string;
}
