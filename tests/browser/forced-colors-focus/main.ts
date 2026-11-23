import { mount } from 'svelte';
import '../../../src/app.css';
import Harness from './Harness.svelte';

const params = new URLSearchParams(location.search);
document.documentElement.dataset.theme = params.get('theme') ?? 'base-dark';
mount(Harness, { target: document.querySelector('#surface')!, props: { dialog: params.get('dialog') === '1' } });
document.documentElement.dataset.ready = 'true';
