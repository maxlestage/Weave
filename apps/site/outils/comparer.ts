/*
 * Compare le site rendu par React (`apps/web/dist`) et celui rendu par Yew
 * (`apps/site/dist-rs`), section par section et langue par langue.
 *
 * ## Pourquoi un outil, et pas un coup d'œil
 *
 * Le portage déplace des milliers de mots — dont ceux des CGU, où chaque
 * caractère engage. Relire deux rendus à l'œil pour s'assurer qu'aucune phrase
 * n'a sauté ne marche pas : on voit ce qu'on cherche, pas ce qui manque.
 *
 * Et la comparaison ne peut pas porter sur le seul texte. Ma première version
 * de `composants.rs` posait `style="color-mix(…)"` sans nom de propriété : les
 * sections alternées perdaient leur fond, et le texte, lui, était intact. Un
 * contrôle textuel aurait déclaré le portage parfait. On compare donc AUSSI la
 * structure : l'imbrication des balises et leurs attributs.
 *
 * ## Pourquoi par section
 *
 * Le portage avance une section à la fois. Comparer les pages entières ne
 * dirait qu'une chose — « elles diffèrent » — pendant tout le portage, et ne
 * deviendrait utile qu'au dernier jour. Comparer par `id` de section donne un
 * verdict par section : celles qui sont portées sont tenues dès maintenant,
 * celles qui restent sont listées.
 *
 *     bun run apps/site/outils/comparer.ts
 *
 * Sortie non nulle si une section portée diverge. C'est fait pour la CI.
 */

const RACINE = new URL("../../../", import.meta.url).pathname;
const REACT = `${RACINE}apps/web/dist`;
const YEW = `${RACINE}apps/site/dist-rs`;

const LANGUES = [
  { code: "fr", prefixe: "" },
  { code: "en", prefixe: "/en" },
  { code: "es", prefixe: "/es" },
] as const;

/* — Le relevé d'une page ———————————————————————————————————————— */

type Ouverture = { readonly genre: "ouvre"; readonly balise: string; readonly attributs: string };
type Fermeture = { readonly genre: "ferme"; readonly balise: string };
type Texte = { readonly genre: "texte"; readonly texte: string };
type Evenement = Ouverture | Fermeture | Texte;

/** Les balises sans contenu : elles n'ont pas de fermeture à attendre. */
const VIDES = new Set([
  "area",
  "base",
  "br",
  "col",
  "embed",
  "hr",
  "img",
  "input",
  "link",
  "meta",
  "source",
  "track",
  "wbr",
]);

/** Les balises dont le contenu n'est pas du texte lu par un visiteur. */
const MUETTES = new Set(["script", "style", "template"]);

/**
 * Normalise une liste de classes.
 *
 * L'ordre des classes utilitaires ne change rien au rendu, et les deux
 * chaînes ne les écrivent pas forcément dans le même ordre : `classes!` de Yew
 * concatène ses morceaux comme il les reçoit. On trie donc avant de comparer,
 * sans quoi l'outil signalerait des différences qui n'en sont pas — et un
 * outil qui crie à tort finit par ne plus être lu.
 */
const normaliserClasses = (valeur: string) => valeur.split(/\s+/).filter(Boolean).sort().join(" ");

/**
 * Normalise une déclaration de style.
 *
 * React sérialise son objet de style sans espace après le deux-points, Yew
 * écrit la chaîne qu'on lui donne. Même déclaration, deux écritures. On
 * compare donc des déclarations, pas des chaînes.
 */
const normaliserStyle = (valeur: string) =>
  valeur
    .split(";")
    .map((d) => d.trim())
    .filter(Boolean)
    .map((d) => {
      const coupure = d.indexOf(":");
      if (coupure < 0) return d.toLowerCase();
      const propriete = d.slice(0, coupure).trim().toLowerCase();
      const contenu = d
        .slice(coupure + 1)
        .trim()
        .replace(/\s+/g, " ");
      return `${propriete}:${contenu}`;
    })
    .sort()
    .join("; ");

/**
 * Décode les entités HTML.
 *
 * Indispensable, et pas cosmétique : React échappe l'apostrophe en `&#x27;`,
 * Yew l'écrit telle quelle. Sans décodage, chaque apostrophe du site — il y en
 * a des centaines — serait signalée comme une divergence, et l'outil
 * deviendrait illisible le jour où il aurait quelque chose à dire.
 */
const NOMMEES: Record<string, string> = {
  amp: "&",
  lt: "<",
  gt: ">",
  quot: '"',
  apos: "'",
  nbsp: "\u00a0",
};

const decoder = (texte: string) =>
  texte.replace(/&(#x[0-9a-f]+|#\d+|[a-z]+);/gi, (entier, corps: string) => {
    const bas = corps.toLowerCase();
    if (bas.startsWith("#x")) return String.fromCodePoint(parseInt(bas.slice(2), 16));
    if (bas.startsWith("#")) return String.fromCodePoint(Number(bas.slice(1)));
    return NOMMEES[bas] ?? entier;
  });

const normaliserAttribut = (nom: string, valeur: string) => {
  if (nom === "class") return normaliserClasses(decoder(valeur));
  if (nom === "style") return normaliserStyle(decoder(valeur));
  return decoder(valeur).trim().replace(/\s+/g, " ");
};

/**
 * Relève une page : la suite de ses balises, de leurs attributs et de son
 * texte, dans l'ordre du document.
 *
 * Les commentaires ne produisent aucun événement — et c'est exactement ce
 * qu'il faut, puisque le HTML de Yew est semé de marqueurs d'hydratation que
 * React n'a pas. Deux morceaux de texte séparés par un marqueur reviennent
 * donc collés, comme ils s'affichent.
 */
async function relever(html: string): Promise<Evenement[]> {
  const evenements: Evenement[] = [];
  let muet = 0;

  const rewriter = new HTMLRewriter();
  rewriter.on("*", {
    element(e) {
      if (MUETTES.has(e.tagName)) {
        muet += 1;
        e.onEndTag(() => {
          muet -= 1;
        });
        return;
      }
      if (muet > 0) return;

      const attributs = [...e.attributes]
        .map(([nom, valeur]) => [nom.toLowerCase(), normaliserAttribut(nom.toLowerCase(), valeur)])
        .filter(([, valeur]) => valeur !== "")
        .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))
        .map(([nom, valeur]) => `${nom}="${valeur}"`)
        .join(" ");

      // Le nom est relevé MAINTENANT, et non dans la fermeture : à ce
      // moment-là, `e` ne désigne plus la balise ouvrante et `e.tagName` vaut
      // `undefined`. Toutes les fermetures s'appelaient donc `undefined`, ce
      // qui n'empêchait pas les deux relevés de se comparer — ils se
      // trompaient pareil — mais rendait le découpage par section incapable de
      // trouver la fin d'une section. Chaque section s'étendait jusqu'au bas
      // de la page, et l'outil signalait des divergences inventées.
      const balise = e.tagName;
      evenements.push({ genre: "ouvre", balise, attributs });
      if (!VIDES.has(balise) && !e.selfClosing) {
        e.onEndTag(() => {
          evenements.push({ genre: "ferme", balise });
        });
      }
    },
    text(t) {
      if (muet > 0) return;
      const precedent = evenements.at(-1);
      // Deux morceaux de suite appartiennent au même passage de texte : ils
      // n'étaient séparés que par un commentaire. On les recolle.
      if (precedent?.genre === "texte") {
        evenements[evenements.length - 1] = {
          genre: "texte",
          texte: precedent.texte + t.text,
        };
      } else {
        evenements.push({ genre: "texte", texte: t.text });
      }
    },
  });

  await rewriter.transform(new Response(html)).text();

  // L'espacement se normalise à la fin, une fois les morceaux recollés : le
  // faire avant écraserait l'espace qui sépare deux mots de part et d'autre
  // d'un marqueur.
  return evenements.flatMap((e) => {
    if (e.genre !== "texte") return [e];
    const texte = decoder(e.texte).replace(/\s+/g, " ").trim();
    return texte === "" ? [] : [{ genre: "texte", texte } as Evenement];
  });
}

/* — Les sections ————————————————————————————————————————————————— */

/** Découpe le relevé en sections, par `id`, fermeture comprise. */
function sections(evenements: Evenement[]): Map<string, Evenement[]> {
  const trouvees = new Map<string, Evenement[]>();

  for (let i = 0; i < evenements.length; i += 1) {
    const e = evenements[i];
    if (e.genre !== "ouvre" || e.balise !== "section") continue;
    const id = /\bid="([^"]*)"/.exec(e.attributs)?.[1];
    if (id === undefined) continue;

    // La fermeture de CETTE section, et non la première rencontrée : une
    // section peut en contenir une autre, et s'arrêter trop tôt comparerait
    // des moitiés de section en déclarant le reste absent.
    let profondeur = 0;
    let fin = evenements.length;
    for (let j = i; j < evenements.length; j += 1) {
      const f = evenements[j];
      if (f.genre === "ouvre" && f.balise === "section") profondeur += 1;
      else if (f.genre === "ferme" && f.balise === "section") {
        profondeur -= 1;
        if (profondeur === 0) {
          fin = j + 1;
          break;
        }
      }
    }
    trouvees.set(id, evenements.slice(i, fin));
  }

  return trouvees;
}

/* — La comparaison ——————————————————————————————————————————————— */

const ecrire = (e: Evenement) =>
  e.genre === "texte"
    ? `« ${e.texte} »`
    : e.genre === "ferme"
      ? `</${e.balise}>`
      : `<${e.balise}${e.attributs ? ` ${e.attributs}` : ""}>`;

/** La première divergence, et son voisinage. Rien de plus : une liste de cent
 * différences toutes issues d'un même décalage ne renseigne pas. */
function diverger(attendu: Evenement[], obtenu: Evenement[]): string[] | null {
  const commun = Math.min(attendu.length, obtenu.length);
  for (let i = 0; i < commun; i += 1) {
    const a = ecrire(attendu[i]);
    const b = ecrire(obtenu[i]);
    if (a !== b) {
      const avant = attendu.slice(Math.max(0, i - 2), i).map(ecrire);
      return [...avant.map((l) => `      … ${l}`), `      React : ${a}`, `      Yew   : ${b}`];
    }
  }
  if (attendu.length !== obtenu.length) {
    const manque = attendu.length > obtenu.length;
    const reste = (manque ? attendu : obtenu).slice(commun, commun + 3).map(ecrire);
    return [
      `      ${manque ? "Yew s'arrête" : "React s'arrête"} après ${commun} nœuds ;`,
      `      ${manque ? "React" : "Yew"} continue par :`,
      ...reste.map((l) => `      … ${l}`),
    ];
  }
  return null;
}

const texteSeul = (evenements: Evenement[]) =>
  evenements.filter((e): e is Texte => e.genre === "texte").map((e) => e.texte);

/* — Le déroulé ——————————————————————————————————————————————————— */

const lire = async (chemin: string) => {
  const fichier = Bun.file(chemin);
  return (await fichier.exists()) ? await fichier.text() : null;
};

let divergences = 0;
let comparees = 0;
const aPorter = new Set<string>();

for (const langue of LANGUES) {
  const chemin = `${langue.prefixe}/index.html`;
  const [reactHtml, yewHtml] = await Promise.all([
    lire(`${REACT}${chemin}`),
    lire(`${YEW}${chemin}`),
  ]);

  console.log(`\n  ${langue.code}  ${chemin}`);

  if (reactHtml === null) {
    console.log("    React n'a rien rendu ici — construire apps/web d'abord.");
    divergences += 1;
    continue;
  }
  if (yewHtml === null) {
    console.log("    Yew n'a rien rendu ici — construire apps/site d'abord.");
    divergences += 1;
    continue;
  }

  const [attendu, obtenu] = await Promise.all([relever(reactHtml), relever(yewHtml)]);
  const sectionsReact = sections(attendu);
  const sectionsYew = sections(obtenu);

  for (const [id, aGauche] of sectionsReact) {
    const aDroite = sectionsYew.get(id);
    if (aDroite === undefined) {
      aPorter.add(id);
      continue;
    }

    comparees += 1;
    const structure = diverger(aGauche, aDroite);
    if (structure === null) {
      console.log(`    #${id} — identique (${aGauche.length} nœuds)`);
      continue;
    }

    divergences += 1;
    // Le texte d'abord : un mot perdu est plus grave qu'un attribut déplacé,
    // et le dire séparément évite de chercher une phrase manquante dans un
    // écart de classes.
    const motsReact = texteSeul(aGauche);
    const motsYew = texteSeul(aDroite);
    const memeTexte = motsReact.join("\u0000") === motsYew.join("\u0000");
    console.log(
      `    #${id} — DIVERGE  (texte ${memeTexte ? "identique" : "DIFFÉRENT"}, structure différente)`,
    );
    if (!memeTexte) {
      const ecart = diverger(
        motsReact.map((texte) => ({ genre: "texte", texte }) as Evenement),
        motsYew.map((texte) => ({ genre: "texte", texte }) as Evenement),
      );
      console.log("      — texte —");
      for (const ligne of ecart ?? []) console.log(ligne);
      console.log("      — structure —");
    }
    for (const ligne of structure) console.log(ligne);
  }

  for (const id of sectionsYew.keys()) {
    if (!sectionsReact.has(id)) {
      divergences += 1;
      console.log(`    #${id} — rendue par Yew seul : React n'a pas cette section.`);
    }
  }
}

console.log(`\n  ${comparees} comparaisons, ${divergences} divergence(s).`);
if (aPorter.size > 0) {
  console.log(`  Sections encore absentes de Yew : ${[...aPorter].sort().join(", ")}.`);
}
if (divergences > 0) process.exit(1);
