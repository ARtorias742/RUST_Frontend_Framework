"use client";

import { motion } from "framer-motion";
import { Blocks, Package, Terminal } from "lucide-react";

import { CopyButton } from "@/components/copy-button";

const CARGO = "cargo add ferrum";
const CLI =
  "cargo install --git https://github.com/ARtorias742/RUST_Frontend_Framework --bin ferrum --locked";

export function Install() {
  return (
    <section id="install" className="border-y border-border/60 bg-secondary/20 py-20">
      <div className="container">
        <div className="mx-auto max-w-2xl text-center">
          <h2 className="text-3xl font-semibold tracking-tight">Install</h2>
          <p className="mt-3 text-muted-foreground">
            The crates are not on crates.io yet. Use the CLI from source, or
            install the VS Code extension from the alpha release.
          </p>
        </div>

        <div className="mt-12 grid gap-4 md:grid-cols-3">
          <motion.div
            initial={{ opacity: 0, y: 12 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.4 }}
            className="rounded-xl border border-border bg-card p-5"
          >
            <Terminal className="h-5 w-5 text-primary" />
            <h3 className="mt-4 font-medium">Ferrum CLI</h3>
            <p className="mt-1 text-sm text-muted-foreground">
              Scaffold, dev server with HMR, build and release gates.
            </p>
            <div className="mt-4 flex items-center gap-2 rounded-lg border border-border bg-background px-3 py-2">
              <code className="min-w-0 flex-1 truncate font-mono text-xs">
                {CLI}
              </code>
              <CopyButton value={CLI} />
            </div>
          </motion.div>

          <motion.div
            initial={{ opacity: 0, y: 12 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.4, delay: 0.08 }}
            className="rounded-xl border border-border bg-card p-5"
          >
            <Blocks className="h-5 w-5 text-primary" />
            <h3 className="mt-4 font-medium">VS Code extension</h3>
            <p className="mt-1 text-sm text-muted-foreground">
              Commands, snippets and an rsx! grammar. Download the{" "}
              <code className="font-mono text-xs">.vsix</code> from Releases,
              then Extensions → ⋯ → Install from VSIX.
            </p>
            <a
              href="https://github.com/ARtorias742/RUST_Frontend_Framework/releases/tag/v0.1.0-alpha"
              target="_blank"
              rel="noreferrer"
              className="mt-4 inline-block text-sm text-primary hover:underline"
            >
              Download the alpha release →
            </a>
            <a
              href="https://github.com/ARtorias742/RUST_Frontend_Framework/releases/download/v0.1.0-alpha/ferrum-0.1.0.vsix"
              target="_blank"
              rel="noreferrer"
              className="mt-2 block text-sm text-primary hover:underline"
            >
              Direct .vsix download →
            </a>
          </motion.div>

          <motion.div
            initial={{ opacity: 0, y: 12 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.4, delay: 0.16 }}
            className="rounded-xl border border-border bg-card p-5"
          >
            <Package className="h-5 w-5 text-primary" />
            <h3 className="mt-4 font-medium">Cargo (soon)</h3>
            <p className="mt-1 text-sm text-muted-foreground">
              Once 0.1 is published, apps depend on the facade only.
            </p>
            <div className="mt-4 flex items-center gap-2 rounded-lg border border-border bg-background px-3 py-2">
              <code className="min-w-0 flex-1 truncate font-mono text-xs">
                {CARGO}
              </code>
              <CopyButton value={CARGO} />
            </div>
          </motion.div>
        </div>

        <p className="mt-8 text-center text-xs text-muted-foreground">
          Alpha: APIs may change. See the status page before you build on it.
        </p>
      </div>
    </section>
  );
}