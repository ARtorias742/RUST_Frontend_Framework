import Link from "next/link";
import { Github } from "lucide-react";

import { Button } from "@/components/ui/button";

export function SiteHeader() {
  return (
    <header className="sticky top-0 z-40 border-b border-border/60 bg-background/70 backdrop-blur">
      <div className="container flex h-16 items-center justify-between">
        <Link href="/" className="flex items-center gap-2.5">
          <span className="grid h-8 w-8 place-items-center rounded-md bg-gradient-to-br from-sky-400 to-indigo-500 font-mono text-sm font-bold text-slate-950">
            Fe
          </span>
          <span className="font-semibold tracking-tight">Ferrum</span>
          <span className="rounded-full border border-amber-500/30 bg-amber-500/10 px-2 py-0.5 text-[11px] font-medium text-amber-300">
            0.1 alpha
          </span>
        </Link>

        <nav className="hidden items-center gap-6 text-sm text-muted-foreground md:flex">
          <Link href="#features" className="hover:text-foreground">
            Features
          </Link>
          <Link href="#install" className="hover:text-foreground">
            Install
          </Link>
          <Link href="#status" className="hover:text-foreground">
            Status
          </Link>
          <a
            href="https://github.com/ARtorias742/RUST_Frontend_Framework"
            target="_blank"
            rel="noreferrer"
            className="hover:text-foreground"
          >
            GitHub
          </a>
        </nav>

        <Button asChild size="sm" variant="outline">
          <a
            href="https://github.com/ARtorias742/RUST_Frontend_Framework/blob/main/README.md"
            target="_blank"
            rel="noreferrer"
          >
            <Github className="h-4 w-4" />
            Star on GitHub
          </a>
        </Button>
      </div>
    </header>
  );
}