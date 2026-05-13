# Project Memory

<!-- Curated long-term memory. Store durable decisions, conventions, preferences, and pitfalls. -->

## Decisions

## Conventions

## Pitfalls

## Context

- Source tests compile `src/lib/*.ts` through `tsconfig.test.json`; if a lib module imports `.svelte` components (for example `src/lib/surfaceLoader.ts`), keep `src/**/*.d.ts` in `tsconfig.test.json` include so the Svelte module declaration from `src/vite-env.d.ts` is available during `npm run test:node`.
