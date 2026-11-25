import { SiteHeader } from "@/components/site-header";
import { Hero } from "@/components/hero";
import { Features } from "@/components/features";
import { Install } from "@/components/install";
import { Status } from "@/components/status";
import { Footer } from "@/components/footer";

export default function Home() {
  return (
    <div className="relative overflow-hidden">
      <div className="pointer-events-none absolute inset-x-0 top-0 h-[520px] grid-bg" />
      <SiteHeader />
      <main>
        <Hero />
        <Features />
        <Install />
        <Status />
      </main>
      <Footer />
    </div>
  );
}