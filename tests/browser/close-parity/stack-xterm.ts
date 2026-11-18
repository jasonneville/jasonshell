import { Terminal as Xterm } from '@xterm/xterm';

// Test-only opt-in for the installed Unicode11 addon. All terminal behavior
// (including key handling and DOM focus) still comes from the real xterm.
export class Terminal extends Xterm {
  constructor(options?: ConstructorParameters<typeof Xterm>[0]) {
    super({ ...options, allowProposedApi: true });
  }
}
