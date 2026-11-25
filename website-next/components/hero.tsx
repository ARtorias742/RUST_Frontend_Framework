"use client";

import Link from "next/link";
import { motion } from "framer-motion";
import { ArrowRight, Play, Terminal } from "lucide-react";

import { Button } from "@/components/ui/button";
import { CopyButton } from "@/components/copy-button";

const INSTALL = "cargo install --git https://github.com/ARtorias742/RUST_Frontend_Framework --bin ferrum --locked";

export function Hero() {
  return (
    <section className="container grid items-center gap-10 py-20 md:grid-cols-[1.1fr_0.9fr] md:py-28">
      <motion.div
        initial={{ opacity: 0, y: 14 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.55, ease: "easeOut" }}
      >
        <div className="mb-5 inline-flex items-center gap-2 rounded-full border border-border bg-card/60 px-3 py-1 text-xs text-muted-foreground">
          <span className="h-1.5 w-1.5 rounded-full bg-emerald-400" />
          Public alpha · 0.1.0 · not production-ready
        </div>

        <h1 className="text-4xl font-semibold tracking-tight sm:text-5xl md:text-6xl">
          Rust, <span className="text-gradient">rendered.</span>
        </h1>

        <p className="mt-5 max-w-xl text-base text-muted-foreground md:text-lg">
          Ferrum is a Rust frontend framework with a React-shaped API:
          components, hooks, SSR, routing, data, scoped styling and DevTools —
          one language, no JS bridge in your components.
        </p>

        <div className="mt-8 flex flex-wrap items-center gap-3">
          <Button asChild size="lg">
            <Link href="#install">
              <Terminal className="h-4 w-4" />
              Install
            </Link>
          </Button>
          <Button asChild size="lg" variant="outline">
            <Link href="#features">
              Explore
              <ArrowRight className="h-4 w-4" />
            </Link>
          </Button>
          <Button asChild size="lg" variant="ghost">
            <a
              href="https://ferrum-rs.github.io/ferrum/demo/"
              target="_blank"
              rel="noreferrer"
            >
              <Play className="h-4 w-4" />
              Live demo
            </a>
          </Button>
        </div>

        <div className="mt-8 flex max-w-xl items-center gap-2 rounded-lg border border-border bg-card/70 px-3 py-2 font-mono text-xs text-muted-foreground">
          <span className="text-emerald-400">$</span>
          <code className="min-w-0 flex-1 truncate">{INSTALL}</code>
          <CopyButton value={INSTALL} />
        </div>
      </motion.div>

      <motion.div
        initial={{ opacity: 0, y: 18 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.6, delay: 0.12, ease: "easeOut" }}
        className="relative"
      >
        <div className="absolute -inset-6 rounded-3xl bg-gradient-to-tr from-sky-500/20 via-blue-500/10 to-indigo-500/20 blur-2xl" />
        <div className="relative overflow-hidden rounded-2xl border border-border bg-card/80 shadow-2xl">
          <div className="flex items-center gap-1.5 border-b border-border px-4 py-3">
            <span className="h-2.5 w-2.5 rounded-full bg-red-400/80" />
            <span className="h-2.5 w-2.5 rounded-full bg-amber-400/80" />
            <span className="h-2.5 w-2.5 rounded-full bg-emerald-400/80" />
            <span className="ml-2 font-mono text-xs text-muted-foreground">
              counter.rs
            </span>
          </div>
          <pre className="overflow-x-auto p-5 font-mono text-[13px] leading-6 text-slate-300">
            <code>{`use ferrum::prelude::*;

#[component]
fn Counter() -> VNode {
    let (count, set) = use_state(|| 0i32);
    rsx! {
        button {
            onclick: move |_| set.update(|c| c + 1),
            "Count: {count}"
        }
    }
}`}</code>
          </pre>
          <div className="border-t border-border bg-secondary/40 px-5 py-3 text-xs text-muted-foreground">
            Same component → web, headless, native.
          </div>
        </div>
      </motion.div>
    </section>
  );
}