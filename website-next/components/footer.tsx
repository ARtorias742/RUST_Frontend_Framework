import Link from "next/link";
import { Github } from "lucide-react";

export function Footer() {
  return (
    <footer className="border-t border-border/60 py-10">
      <div className="container flex flex-col items-start justify-between gap-6 sm:flex-row sm:items-center">
        <div>
          <div className="flex items-center gap-2 font-medium">
            <span className="grid h-7 w-7 place-items-center rounded-md bg-gradient-to-br from-sky-400 to-indigo-500 font-mono text-xs font-bold text-slate-950">
              Fe
            </span>
            Ferrum
          </div>
          <p className="mt-2 text-sm text-muted-foreground">
            A Rust frontend framework. Apache-2.0.
          </p>
        </div>

        <nav className="flex flex-wrap items-center gap-5 text-sm text-muted-foreground">
          <Link href="#features" className="hover:text-foreground">
            Features
          </Link>
          <Link href="#install" className="hover:text-foreground">
            Install
          </Link>
          <a
            href="https://github.com/ferrum-rs/ferrum"
            target="_blank"
            rel="noreferrer"
            className="inline-flex items-center gap-1.5 hover:text-foreground"
          >
            <Github className="h-4 w-4" />
            GitHub
          </a>
          <a
            href="https://github.com/ferrum-rs/ferrum/blob/main/SECURITY.md"
            target="_blank"
            rel="noreferrer"
            className="hover:text-foreground"
          >
            Security
          </a>
        </nav>
      </div>
      <div className="container mt-8 text-xs text-muted-foreground">
        0.1.0-alpha · not production-ready · APIs may change.
      </div>
    </footer>
  );
}