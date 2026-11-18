import '../../../src/app.css';
import { mount } from 'svelte';
import Settings from '../../../src/components/SettingsPanelSurface.svelte';
import Stack from '../../../src/components/StackPopupSurface.svelte';
import ControlPlane from '../../../src/components/ControlPlaneSurface.svelte';
import { ledger } from './bridge';
import { applyShellTheme, storedShellThemeId } from '../../../src/lib/themes';
import { applyShellPreferences, storedShellPreferences } from '../../../src/lib/shellPreferences';

applyShellTheme(storedShellThemeId(), { storage: null });
applyShellPreferences(storedShellPreferences(), { storage: null, dispatch: false });
const target = document.getElementById('app')!;
const surface=new URLSearchParams(location.search).get('surface');
mount(surface === 'stack' ? Stack : surface === 'control' ? ControlPlane : Settings, { target });
Object.assign(window, { exitFixture: {
  ledger, hide: () => { target.hidden = true; }, show: () => { target.hidden = false; },
} });
