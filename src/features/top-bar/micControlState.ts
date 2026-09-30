import type { SpeechSessionNonce, SpeechStatusEvent, SpeechStatusKind } from '../../ipc/events.js';

export type MicControlState =
  | 'idle'
  | 'starting'
  | 'recording'
  | 'stopping'
  | 'transcribing'
  | 'copied'
  | 'error';

export const SPEECH_ERROR_CODES = [
  'capture-stream-error',
  'capture-callback-loss',
  'capture-empty',
  'capture-short',
  'model-load-failed',
  'asr-input-invalid',
  'asr-failed',
  'transcript-empty',
  'clipboard-failed',
  'clipboard-sta-unavailable',
  'clipboard-queue-full',
  'clipboard-invalid-text',
  'clipboard-timeout',
  'clipboard-publish-rejected',
  'paste-target-unavailable',
  'paste-target-changed',
  'paste-focus-denied',
  'paste-input-rejected',
  'timeout',
  'state-race'
] as const;

export type SpeechErrorCode = (typeof SPEECH_ERROR_CODES)[number];

export interface MicControlModel {
  state: MicControlState;
  nonce: SpeechSessionNonce | null;
  announcement: string;
  errorCode: SpeechErrorCode | null;
}

export const INITIAL_MIC_CONTROL_MODEL: MicControlModel = {
  state: 'idle',
  nonce: null,
  announcement: '',
  errorCode: null
};

export const MIC_CONTROL_PRESENTATION = {
  idle: { label: 'Start recording and transcribe', pressed: false, disabled: false },
  starting: { label: 'Starting speech recording', pressed: false, disabled: true },
  recording: { label: 'Stop recording and transcribe', pressed: true, disabled: false },
  stopping: { label: 'Transcribing speech', pressed: false, disabled: true },
  transcribing: { label: 'Transcribing speech', pressed: false, disabled: true },
  copied: { label: 'Speech pasted into original target', pressed: false, disabled: true },
  error: { label: 'Speech transcription failed; retry recording', pressed: false, disabled: false }
} as const satisfies Record<MicControlState, {
  label: string;
  pressed: boolean;
  disabled: boolean;
}>;

export function beginSpeechStart(model: MicControlModel): MicControlModel {
  if (model.state !== 'idle' && !(model.state === 'error' && model.nonce === null)) return model;
  return {
    state: 'starting',
    nonce: null,
    announcement: 'Starting speech recording',
    errorCode: null
  };
}

export function acceptSpeechStart(
  model: MicControlModel,
  response: { nonce: SpeechSessionNonce; status: SpeechStatusKind }
): MicControlModel {
  if (model.state !== 'starting' || response.status !== 'recording') return model;
  return {
    state: 'recording',
    nonce: response.nonce,
    announcement: 'Recording speech',
    errorCode: null
  };
}

export function beginSpeechStop(model: MicControlModel): MicControlModel {
  if (model.state !== 'recording') return model;
  return { ...model, state: 'stopping', announcement: 'Stopping recording', errorCode: null };
}

export function acceptSpeechStop(
  model: MicControlModel,
  response: { nonce: SpeechSessionNonce | null; status: SpeechStatusKind }
): MicControlModel {
  if (model.state !== 'stopping' || response.nonce !== model.nonce || response.status !== 'transcribing') {
    return model;
  }
  return { ...model, state: 'transcribing', announcement: 'Transcribing speech', errorCode: null };
}

export function failSpeechCommand(
  model: MicControlModel,
  errorCode: SpeechErrorCode = 'state-race'
): MicControlModel {
  return speechFailureModel(model.nonce, errorCode);
}

export function normalizeSpeechErrorCode(error: string | null | undefined): SpeechErrorCode {
  return SPEECH_ERROR_CODES.includes(error as SpeechErrorCode) ? error as SpeechErrorCode : 'state-race';
}

export function normalizeSpeechCommandError(error: unknown): SpeechErrorCode {
  if (typeof error === 'string') return normalizeSpeechErrorCode(error);
  if (
    typeof error === 'object'
    && error !== null
    && 'message' in error
    && typeof error.message === 'string'
  ) {
    return normalizeSpeechErrorCode(error.message);
  }
  return 'state-race';
}

export function speechControlLabel(model: MicControlModel): string {
  return model.state === 'error' && model.errorCode !== null
    ? model.announcement
    : MIC_CONTROL_PRESENTATION[model.state].label;
}

function speechFailureModel(
  nonce: SpeechSessionNonce | null,
  errorCode: SpeechErrorCode
): MicControlModel {
  return {
    state: 'error',
    nonce,
    errorCode,
    announcement: `Speech transcription failed: ${errorCode}`
  };
}

export async function settleSpeechCommand<T>(
  command: Promise<T>,
  isDisposed: () => boolean,
  onResolved: (response: T) => void,
  onRejected: (error: unknown) => void
): Promise<void> {
  try {
    const response = await command;
    if (isDisposed()) return;
    onResolved(response);
  } catch (error) {
    if (isDisposed()) return;
    onRejected(error);
  }
}

export function reduceSpeechEvent(model: MicControlModel, event: SpeechStatusEvent): MicControlModel {
  if (event.status === 'idle') {
    return (model.state === 'copied' || model.state === 'error') && event.nonce === model.nonce
      ? INITIAL_MIC_CONTROL_MODEL
      : model;
  }
  if (model.nonce === null || event.nonce !== model.nonce) return model;
  if (event.status === 'recording') {
    return { ...model, state: 'recording', announcement: 'Recording speech', errorCode: null };
  }
  if (event.status === 'transcribing') {
    return {
      ...model,
      state: 'transcribing',
      announcement: event.finalizationReason === 'recording_cap'
        ? 'Recording limit reached; finishing dictation.'
        : 'Transcribing speech',
      errorCode: null
    };
  }
  if (event.status === 'copied') {
    return { ...model, state: 'copied', announcement: 'Speech pasted into original target', errorCode: null };
  }
  return speechFailureModel(model.nonce, normalizeSpeechErrorCode(event.error));
}
