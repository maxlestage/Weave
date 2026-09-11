/** Briques d'interface partagées par les sections du site. */
import type { ReactNode } from "react";

/** Les six fils de la palette, adressables par numéro. */
export type Fil = 1 | 2 | 3 | 4 | 5 | 6;

export const filVar = (fil: Fil) => `var(--fil-${fil})`;

/** Aplat très pâle d'un fil, utilisable derrière du texte courant. */
export const teinte = (fil: Fil, pourcentage = 8) =>
  `color-mix(in oklab, ${filVar(fil)} ${pourcentage}%, var(--fond))`;

export function Section({
  id,
  titre,
  chapeau,
  fil,
  alterne = false,
  children,
}: {
  id: string;
  titre: string;
  chapeau?: string;
  /** Couleur de la section : chacune a la sienne. */
  fil: Fil;
  alterne?: boolean;
  children: ReactNode;
}) {
  return (
    <section
      id={id}
      aria-labelledby={`${id}-titre`}
      className="px-5 py-16 sm:px-8 sm:py-24"
      style={alterne ? { background: teinte(fil, 7) } : undefined}
    >
      <div className="mx-auto w-full max-w-5xl">
        {/* Un court trait de couleur annonce la section. */}
        <span
          aria-hidden="true"
          className="mb-5 block h-1.5 w-16 rounded-full"
          style={{ background: filVar(fil) }}
        />

        <h2
          id={`${id}-titre`}
          className="text-3xl leading-tight font-semibold tracking-tight sm:text-4xl"
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
  fil,
  accentuee = false,
  children,
}: {
  fil: Fil;
  accentuee?: boolean;
  children: ReactNode;
}) {
  return (
    <div
      className="rounded-3xl p-6 sm:p-7"
      style={{
        background: "var(--carte)",
        border: `2px solid ${accentuee ? filVar(fil) : "var(--bordure)"}`,
        boxShadow: accentuee
          ? `0 0 0 5px color-mix(in oklab, ${filVar(fil)} 16%, transparent)`
          : undefined,
      }}
    >
      {children}
    </div>
  );
}

export function Etiquette({ fil, children }: { fil: Fil; children: ReactNode }) {
  return (
    <span
      className="inline-block rounded-full px-3.5 py-1.5 text-xs font-bold tracking-wide uppercase"
      style={{ background: filVar(fil), color: "var(--sur-accent)" }}
    >
      {children}
    </span>
  );
}

/** Pastille numérotée, pour les listes ordonnées. */
export function Pastille({ fil, children }: { fil: Fil; children: ReactNode }) {
  return (
    <span
      className="flex h-11 w-11 shrink-0 items-center justify-center rounded-2xl text-base font-bold tabular-nums"
      style={{ background: filVar(fil), color: "var(--sur-accent)" }}
      aria-hidden="true"
    >
      {children}
    </span>
  );
}
