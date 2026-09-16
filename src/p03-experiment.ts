import { mount } from 'svelte';
import TextEditorProjectionExperiment from './components/TextEditorProjectionExperiment.svelte';

const target = document.getElementById('app');
if (!target) throw new Error('P03 experiment mount target missing');
mount(TextEditorProjectionExperiment, { target });
