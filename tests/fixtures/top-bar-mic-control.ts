import '../../src/app.css';
import '../../src/components/TopBar.css';
import { mount } from 'svelte';
import TopBarMicControl from '../../src/components/TopBarMicControl.svelte';

const target = document.getElementById('app');

if (!target) throw new Error('Missing fixture mount target');

mount(TopBarMicControl, { target });
