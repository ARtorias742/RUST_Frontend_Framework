# Ferrum website

Marketing site for Ferrum, built with **Next.js 14 (App Router)**,
**Tailwind CSS + shadcn/ui**, and **Framer Motion**.

> Alpha site for a 0.1 framework. The copy is intentionally honest about the
> alpha status.

## Run

```bash
cd website-next
npm install
npm run dev        # http://localhost:3000
```

## Build

```bash
npm run build
npm start
```

## Structure

```text
app/                 layout, page, globals.css
components/ui/       shadcn/ui primitives (button)
components/          hero, features, install, status, header, footer
lib/utils.ts         cn()
tailwind.config.ts   shadcn design tokens
components.json      shadcn CLI config
```

## shadcn/ui

The base is shadcn/ui (“new-york” style, CSS variables). Add more components
with the CLI:

```bash
npx shadcn@latest add dialog dropdown-menu tabs
```

New primitives land in `components/ui/`. Keep the `cn()` helper and the CSS
variables in `app/globals.css`.

## Framer Motion

Client components use `motion` from `framer-motion`:

```tsx
"use client";
import { motion } from "framer-motion";

<motion.div
  initial={{ opacity: 0, y: 12 }}
  whileInView={{ opacity: 1, y: 0 }}
  viewport={{ once: true }}
  transition={{ duration: 0.45 }}
/>
```

The hero, feature grid and install cards use this pattern.

## Deploy

The site is a standard Next.js app. Deploy to Vercel, or build and host
`out/` (add `output: "export"` to `next.config.mjs` for a fully static
build). The live docs and demo are expected at:

- `/docs/` — mdbook site
- `/demo/` — `examples/realworld` wasm app

Both are deployed by `.github/workflows/pages.yml`.