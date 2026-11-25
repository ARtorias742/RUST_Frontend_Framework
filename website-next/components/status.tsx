import Link from "next/link";
import { AlertTriangle } from "lucide-react";

const ITEMS = [
  ["Version", "0.1.0-alpha"],
  ["Components", "30"],
  ["Conformance", "20 / 20"],
  ["Demo bundle", "192 KB gzip"],
];

export function Status() {
  return (
    <section id="status" className="container py-20">
      <div className="grid gap-10 md:grid-cols-[0.9fr_1.1fr] md:items-center">
        <div>
          <h2 className="text-3xl font-semibold tracking-tight">
            An honest alpha
          </h2>
          <p className="mt-3 text-muted-foreground">
            The architecture is in place. The API is not frozen. The native
            backend is a scene graph, not a window yet, and the browser matrix
            is still being verified.
          </p>
          <Link
            href="https://github.com/ARtorias742/RUST_Frontend_Framework/blob/main/README.md"
            className="mt-5 inline-block text-sm text-primary hover:underline"
          >
            Read the full status and known gaps →
          </Link>
        </div>

        <div className="rounded-xl border border-amber-500/30 bg-amber-500/5 p-5">
          <div className="flex items-center gap-2 text-amber-300">
            <AlertTriangle className="h-4 w-4" />
            <span className="text-sm font-medium">
              Do not ship Ferrum to end users yet
            </span>
          </div>
          <div className="mt-5 grid grid-cols-2 gap-4 sm:grid-cols-4">
            {ITEMS.map(([label, value]) => (
              <div key={label}>
                <div className="text-lg font-semibold text-foreground">
                  {value}
                </div>
                <div className="text-xs text-muted-foreground">{label}</div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}