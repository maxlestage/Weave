/*
 * Charpente des pages juridiques.
 *
 * Ces pages se lisent, elles ne se parcourent pas : une colonne étroite, des
 * titres numérotés, un sommaire en tête. Elles réutilisent l'en-tête et le pied
 * du site pour qu'on sache toujours où l'on est, mais elles n'empruntent pas
 * ses cartes colorées — un texte contractuel n'est pas une section marketing.
 */

import type { ReactNode } from "react";
import { PiedDePage } from "../sections/PiedDePage.tsx";

const LIENS = [
  { href: "/#principe", texte: "Le principe" },
  { href: "/#deroule", texte: "Comment ça se passe" },
  { href: "/#offres", texte: "Offres" },
] as const;

/**
 * En-tête des pages juridiques : sans état, donc sans JavaScript.
 *
 * Celui de l'accueil ouvre un menu au téléphone, ce qui suppose React côté
 * client. Ces pages-ci sont pré-rendues et n'embarquent aucun script : les
 * trois liens tiennent sur une ligne qui passe à la ligne, et le menu
 * escamotable n'a plus de raison d'être.
 */
function EnteteStatique() {
  return (
    <header>
      <div className="tissage h-1.5" aria-hidden="true" />
      <div className="mx-auto flex w-full max-w-5xl flex-wrap items-center justify-between gap-x-6 gap-y-2 px-5 py-3 sm:px-8">
        <a href="/" className="flex items-center gap-2.5 font-semibold">
          <Logo />
          <span
            className="text-xl font-bold tracking-tight"
            style={{ fontFamily: "var(--font-titre)" }}
          >
            Weave
          </span>
        </a>
        <nav aria-label="Navigation principale" className="flex flex-wrap gap-x-6 gap-y-1 text-sm">
          {LIENS.map((lien) => (
            <a key={lien.href} href={lien.href} className="hover:underline underline-offset-4">
              {lien.texte}
            </a>
          ))}
        </nav>
      </div>
    </header>
  );
}

function Logo() {
  return (
    <svg width="26" height="26" viewBox="0 0 32 32" aria-hidden="true">
      <g strokeWidth="3" strokeLinecap="round" fill="none">
        <path d="M7 5v22" stroke="var(--fil-1)" />
        <path d="M16 5v22" stroke="var(--fil-4)" />
        <path d="M25 5v22" stroke="var(--fil-6)" />
      </g>
      <g strokeWidth="2.4" strokeLinecap="round" fill="none">
        <path d="M4 12c4 3 8 3 12 0s8-3 12 0" stroke="var(--fil-2)" />
        <path d="M4 21c4 3 8 3 12 0s8-3 12 0" stroke="var(--fil-5)" />
      </g>
    </svg>
  );
}

export interface Article {
  readonly id: string;
  readonly titre: string;
  readonly contenu: ReactNode;
}

/**
 * Ce que seul l'éditeur peut renseigner : raison sociale, adresse, numéro
 * d'immatriculation, hébergeur.
 *
 * Le marqueur est volontairement voyant. Une mention légale incomplète expose
 * son éditeur, et un gabarit qui se fond dans le texte finit par être publié
 * tel quel — c'est le mode de défaillance qu'on cherche à rendre impossible.
 */
export function AC({ children }: { children: ReactNode }) {
  return (
    <mark
      className="rounded px-1.5 py-0.5 text-[0.9em] font-semibold"
      style={{ background: "color-mix(in oklab, var(--fil-3) 22%, transparent)", color: "inherit" }}
      title="À compléter avant publication"
    >
      {children}
    </mark>
  );
}

export function Page({
  titre,
  chapeau,
  miseAJour,
  articles,
  apres,
}: {
  titre: string;
  chapeau: string;
  miseAJour: string;
  articles: readonly Article[];
  apres?: ReactNode;
}) {
  return (
    <>
      <a
        href="#contenu"
        className="sr-only focus:not-sr-only focus:absolute focus:left-4 focus:top-4 focus:z-[100] focus:rounded-full focus:px-4 focus:py-2"
        style={{ background: "var(--accent)", color: "var(--sur-accent)" }}
      >
        Aller au contenu
      </a>

      <EnteteStatique />

      <main id="contenu" className="px-5 pt-12 pb-20 sm:px-8">
        <div className="mx-auto w-full max-w-2xl">
          <nav aria-label="Fil d'Ariane" className="text-sm" style={{ color: "var(--texte-doux)" }}>
            <a href="/" className="hover:underline underline-offset-4">
              Accueil
            </a>
            <span aria-hidden="true"> › </span>
            <span>{titre}</span>
          </nav>

          <h1
            className="mt-5 text-[2rem] leading-tight font-bold tracking-tight sm:text-5xl"
            style={{ fontFamily: "var(--font-titre)" }}
          >
            {titre}
          </h1>

          <p className="mt-5 text-lg leading-relaxed">{chapeau}</p>

          <p className="mt-4 text-sm" style={{ color: "var(--texte-doux)" }}>
            Dernière mise à jour : {miseAJour}
          </p>

          <div className="tissage mt-8 h-1.5 rounded-full" aria-hidden="true" />

          <nav aria-labelledby="sommaire" className="mt-8">
            <h2 id="sommaire" className="text-sm font-semibold tracking-wide uppercase">
              Sommaire
            </h2>
            <ol className="mt-3 space-y-1.5 text-sm">
              {articles.map((article, index) => (
                <li key={article.id}>
                  <a href={`#${article.id}`} className="hover:underline underline-offset-4">
                    <span className="tabular-nums" style={{ color: "var(--texte-doux)" }}>
                      {index + 1}.
                    </span>{" "}
                    {article.titre}
                  </a>
                </li>
              ))}
            </ol>
          </nav>

          <div className="mt-12 space-y-11">
            {articles.map((article, index) => (
              <section key={article.id} id={article.id} className="scroll-mt-24">
                <h2 className="text-xl font-bold sm:text-2xl" style={{ color: "var(--fil-4)" }}>
                  <span className="tabular-nums" style={{ color: "var(--texte-doux)" }}>
                    {index + 1}.
                  </span>{" "}
                  {article.titre}
                </h2>
                <div className="prose-weave mt-3.5 leading-relaxed">{article.contenu}</div>
              </section>
            ))}
          </div>

          {apres}
        </div>
      </main>

      <PiedDePage />
    </>
  );
}

/**
 * Un tableau lisible au téléphone : il défile seul plutôt que de déborder.
 *
 * Le conteneur qui défile est FOCALISABLE, et porte un nom. Sans cela, ce qui
 * dépasse à droite n'était atteignable qu'à la souris ou au doigt : personne
 * ne peut faire défiler au clavier une zone qui ne prend pas le focus, et les
 * colonnes cachées d'un tableau de tarifs devenaient illisibles pour qui
 * navigue au clavier. Le nom vient du titre : une zone focalisable et muette
 * ne dit pas ce qu'on vient d'atteindre.
 */
export function Tableau({
  entetes,
  lignes,
  titre,
}: {
  entetes: readonly string[];
  lignes: readonly (readonly ReactNode[])[];
  /** Ce que le tableau montre, pour qui l'atteint au clavier. */
  titre?: string;
}) {
  return (
    <div
      className="-mx-5 mt-4 overflow-x-auto px-5 sm:mx-0 sm:px-0"
      tabIndex={0}
      role="group"
      aria-label={titre ?? "Tableau"}
    >
      <table className="w-full min-w-[34rem] border-collapse text-sm">
        <thead>
          <tr>
            {entetes.map((entete) => (
              <th
                key={entete}
                scope="col"
                className="py-2 pr-4 text-left align-bottom font-semibold"
                style={{ borderBottom: "2px solid var(--bordure)" }}
              >
                {entete}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {lignes.map((ligne, index) => (
            <tr key={index}>
              {ligne.map((cellule, colonne) => (
                <td
                  key={colonne}
                  className="py-2.5 pr-4 align-top"
                  style={{ borderBottom: "1px solid var(--bordure)" }}
                >
                  {cellule}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
