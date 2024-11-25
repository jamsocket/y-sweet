import { Hero } from "./Hero";
import { Presence } from "./Presence";
import { Footer } from "./Footer";
import { Todos } from "./Todos";

export function App() {
  return (
    <div className="min-h-screen flex flex-col justify-between max-w-[60rem] mx-auto before:content-[''] before:block before:absolute before:top-0 before:left-0 before:right-0 before:border-t-4 before:border-[#fc5c86]">
      <header className="flex flex-col gap-8 p-12">
        <Hero />
      </header>

      <Presence />

      <Todos />
      <Footer />
    </div>
  );
}
