"use client";

import { motion } from "framer-motion";
import {
  Boxes,
  Gauge,
  Palette,
  ShieldCheck,
  Workflow,
  Wrench,
} from "lucide-react";

const FEATURES = [
  {
    icon: Boxes,
    title: "Small explicit core",
    body: "A tiny VNode IR. Web, headless and native are replaceable backends over one reconciler.",
  },
  {
    icon: Workflow,
    title: "Hooks that behave",
    body: "State, effects, memo, refs, context, reducers and IDs — with a 20-check conformance suite.",
  },
  {
    icon: Gauge,
    title: "Fast where it counts",
    body: "~1.9 ms per 1k-child keyed update, ~2.8 ms to mount 1k rows, 0.08% heap growth over 10k updates.",
  },
  {
    icon: Palette,
    title: "Scoped styling",
    body: "Hash-scoped CSS, design tokens, dark mode and RTL. Tailwind interop without a runtime.",
  },
  {
    icon: ShieldCheck,
    title: "Safe by default",
    body: "No eval, escaped SSR, an allowlist HTML sanitizer and nonce-based CSP primitives.",
  },
  {
    icon: Wrench,
    title: "DevTools built in",
    body: "Component inspector, state timeline, network panel and an in-app overlay.",
  },
];

export function Features() {
  return (
    <section id="features" className="container py-20">
      <div className="mx-auto max-w-2xl text-center">
        <h2 className="text-3xl font-semibold tracking-tight">
          Everything the 80% needs
        </h2>
        <p className="mt-3 text-muted-foreground">
          Router, data, styling, 30 components and DevTools — without leaving
          the Rust crate graph.
        </p>
      </div>

      <div className="mt-12 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {FEATURES.map((feature, i) => (
          <motion.div
            key={feature.title}
            initial={{ opacity: 0, y: 14 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true, margin: "-80px" }}
            transition={{ duration: 0.45, delay: i * 0.06 }}
            className="group rounded-xl border border-border bg-card/60 p-5 transition-colors hover:border-primary/40 hover:bg-card"
          >
            <div className="mb-4 inline-grid h-10 w-10 place-items-center rounded-lg border border-border bg-secondary/60 text-primary">
              <feature.icon className="h-5 w-5" />
            </div>
            <h3 className="font-medium">{feature.title}</h3>
            <p className="mt-2 text-sm leading-6 text-muted-foreground">
              {feature.body}
            </p>
          </motion.div>
        ))}
      </div>
    </section>
  );
}