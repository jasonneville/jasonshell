import './app.css';
import { mount } from 'svelte';
import App from './App.svelte';
import { applyShellPreferences, storedShellPreferences } from './lib/shellPreferences';
import { applyShellTheme, storedShellThemeId } from './lib/themes';

const target = document.getElementById('app');

if (!target) {
  throw new Error('JasonShell could not find the #app mount target');
}

applyShellTheme(storedShellThemeId(), { storage: null });
applyShellPreferences(storedShellPreferences(), { storage: null, dispatch: false });

const app = mount(App, {
  target
});

export default app;
