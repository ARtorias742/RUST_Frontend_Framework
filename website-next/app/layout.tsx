import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "Ferrum — Rust, rendered",
  description:
    "Ferrum is a Rust frontend framework with a React-shaped API: components, hooks, SSR, routing, data, styling and DevTools. 0.1 alpha.",
  openGraph: {
    title: "Ferrum — Rust, rendered",
    description: "A Rust frontend framework. 0.1.0-alpha.",
    type: "website",
  },
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" className="dark">
      <body className="min-h-screen bg-background font-sans text-foreground">
        {children}
      </body>
    </html>
  );
}