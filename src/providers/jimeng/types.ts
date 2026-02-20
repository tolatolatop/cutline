export interface JimengGenerateParams {
  providerName: string;
  profileName: string;
  prompt: string;
  model?: string;
  ratio?: string;
  negativePrompt?: string;
  imageCount?: number;
}

export interface JimengGenerateResult {
  historyId: string;
  submitId: string;
}

export interface JimengTaskStatusParams {
  providerName: string;
  profileName: string;
  historyIds: string[];
}

export interface JimengCreditBalanceParams {
  providerName: string;
  profileName: string;
}

export interface JimengCreditInfo {
  giftCredit: number;
  purchaseCredit: number;
  vipCredit: number;
}
