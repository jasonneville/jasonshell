import { invoke } from '@tauri-apps/api/core';
import { IPC_COMMANDS } from '../ipc/commands.js';

export const LOCAL_AUTOMATION_OPT_IN_FLAG = '--allow-local-automation';
export const AUTOMATION_FORWARDING_STATUS = 'planned-not-wired';

export type AutomationActionKind = 'help' | 'showSearch' | 'listProviders' | 'activateWorkspace' | 'deleteWorkspace';
export type AutomationSecurityLevel = 'readOnly' | 'mutating' | 'destructive';
export type AutomationSource = 'localCli' | 'singleInstanceForward';

export interface AutomationCliParseRequest {
  args: string[];
}

export interface AutomationAction {
  kind: AutomationActionKind;
  target: string | null;
}

export interface AutomationSecurityBoundary {
  localAutomationEnabled: boolean;
  authenticated: boolean;
  userPresent: boolean;
  destructiveConfirmation: string | null;
}

export interface AutomationRequest {
  source: AutomationSource;
  action: AutomationAction;
  boundary: AutomationSecurityBoundary;
}

export interface AutomationValidation {
  accepted: boolean;
  action: AutomationActionKind;
  securityLevel: AutomationSecurityLevel;
  forwardingStatus: typeof AUTOMATION_FORWARDING_STATUS;
}

export interface SingleInstanceForwardingContract {
  status: typeof AUTOMATION_FORWARDING_STATUS;
  transport: string;
  acceptsArgvOnly: boolean;
  requiresLocalOptIn: boolean;
  requiresAuthenticatedDestructiveActions: boolean;
  executesForwardedPayloads: boolean;
  arbitraryPluginExecutionAllowed: boolean;
}

export const AUTOMATION_COMMANDS = {
  parseCli: IPC_COMMANDS.parseAutomationCli,
  validateRequest: IPC_COMMANDS.validateAutomationRequest,
  forwardingContract: IPC_COMMANDS.getSingleInstanceForwardingContract
} as const;

export function destructiveAutomationConfirmation(action: AutomationAction): string {
  if (action.kind !== 'deleteWorkspace' || !action.target) {
    throw new Error('Only workspace deletion has a destructive confirmation phrase');
  }
  return `delete-workspace:${action.target}`;
}

export function assertSafeForwardingContract(contract: SingleInstanceForwardingContract): void {
  if (contract.status !== AUTOMATION_FORWARDING_STATUS) {
    throw new Error(`Unexpected automation forwarding status: ${contract.status}`);
  }
  if (!contract.acceptsArgvOnly || !contract.requiresLocalOptIn) {
    throw new Error('Automation forwarding must remain argv-only and locally opted-in');
  }
  if (contract.executesForwardedPayloads || contract.arbitraryPluginExecutionAllowed) {
    throw new Error('Automation forwarding must not execute forwarded payloads or plugins');
  }
}

export async function parseAutomationCli(args: string[]): Promise<AutomationRequest> {
  return invoke<AutomationRequest>(AUTOMATION_COMMANDS.parseCli, { request: { args } });
}

export async function validateAutomationRequest(request: AutomationRequest): Promise<AutomationValidation> {
  return invoke<AutomationValidation>(AUTOMATION_COMMANDS.validateRequest, { request });
}

export async function getSingleInstanceForwardingContract(): Promise<SingleInstanceForwardingContract> {
  const contract = await invoke<SingleInstanceForwardingContract>(AUTOMATION_COMMANDS.forwardingContract);
  assertSafeForwardingContract(contract);
  return contract;
}
