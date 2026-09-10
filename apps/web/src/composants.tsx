/** Briques d'interface partagées par les sections du site. */
import type { ReactNode } from "react";

export function Section({
  id,
  titre,
  chapeau,
  alterne = false,
  children,
}: {
  id: string;
  titre: string;
  chapeau?: string;
  alterne?: boolean;
  children: ReactNode;
}) {
  return (
    <section
      id={id}
      aria-labelledby={`${id}-titre`}
      className="px-5 py-16 sm:px-8 sm:py-24"
      style={alterne ? { background: "var(--fond-alterne)" } : undefined}
    >
      <div className="mx-auto w-full max-w-5xl">
        <h2
          id={`${id}-titre`}
          className="text-3xl leading-tight tracking-tight sm:text-4xl"
          style={{ fontFamily: "var(--font-titre)" }}
        >
          {titre}
        </h2>
        {chapeau !== undefined && (
          <p
            className="mt-4 max-w-2xl text-lg leading-relaxed"
            style={{ color: "var(--texte-doux)" }}
          >
            {chapeau}
          </p>
        )}
        <div className="mt-10">{children}</div>
      </div>
    </section>
  );
}

export function Carte({
  children,
  accentuee = false,
}: {
  children: ReactNode;
  accentuee?: boolean;
}) {
  return (
    <div
      className="rounded-2xl p-6 sm:p-7"
      style={{
        background: "var(--carte)",
        border: `1px solid ${accentuee ? "var(--accent)" : "var(--bordure)"}`,
        boxShadow: accentuee
          ? "0 0 0 3px color-mix(in oklab, var(--accent) 14%, transparent)"
          : undefined,
      }}
    >
      {children}
    </div>
  );
}

export function Etiquette({ children }: { children: ReactNode }) {
  return (
    <span
      className="inline-block rounded-full px-3 py-1 text-xs font-semibold tracking-wide uppercase"
      style={{
        background: "color-mix(in oklab, var(--accent) 16%, transparent)",
        color: "var(--accent)",
      }}
    >
      {children}
    </span>
  );
}
