/*
 * Les deux chaînes refusent-elles les mêmes mises en ligne ?
 *
 * ## Pourquoi cela se vérifie à part
 *
 * `comparer.ts` rapproche ce que les deux chaînes PRODUISENT. Mais la moitié de
 * la valeur de `build.ts` est dans ce qu'elle refuse de produire : une origine
 * qui ne mène nulle part, une adresse non chiffrée, des mentions légales
 * incomplètes qui partiraient pour un vrai domaine. Une page jamais écrite ne
 * se compare pas.
 *
 * Or ces refus sont exactement ce qu'on ne peut pas éprouver autrement. Avec
 * `SITE_ORIGINE` posée, les quinze valeurs d'`identite.ts` manquent encore et
 * la construction s'arrête — donc les branches « avec origine » (canonique
 * absolue, `hreflang`, `og:url`, plan du site) ne se construisent PAS, et leur
 * sortie ne se compare pas. Ce qui se compare, c'est le refus.
 *
 * C'est aussi la garde la plus sérieuse du lot : l'article 6-III de la LCEN
 * rend les mentions légales obligatoires, et leur absence est pénalement
 * sanctionnée. Si la chaîne Rust remplace la chaîne React en oubliant ce
 * refus, on perd la seule chose qui empêche une mise en ligne sans SIREN.
 *
 *     bun run apps/site/outils/comparer-les-refus.ts
 */

const RACINE = new URL("../../../", import.meta.url).pathname;

/** Ce qu'on attend d'un refus : qu'il ait lieu, et qu'il dise pourquoi. */
type Cas = {
  readonly origine: string;
  readonly pourquoi: string;
  /** Un fragment que les DEUX chaînes doivent écrire. */
  readonly attendu: string;
};

const CAS: readonly Cas[] = [
  {
    origine: "https://weave.app",
    pourquoi: "le domaine de remplacement du dépôt, qui ne répond pas",
    attendu: "domaine de remplacement",
  },
  {
    origine: "https://www.weave.app",
    pourquoi: "un sous-domaine du domaine de remplacement",
    attendu: "domaine de remplacement",
  },
  {
    origine: "http://exemple.fr",
    pourquoi: "une adresse non chiffrée",
    attendu: "https",
  },
  {
    origine: "https://exemple.fr",
    pourquoi: "les quinze mentions légales manquent encore",
    attendu: "mentions légales manquent",
  },
];

/** Lance une commande et rend son code de sortie et tout ce qu'elle a écrit. */
async function lancer(
  commande: readonly string[],
  origine: string,
): Promise<{ code: number; texte: string }> {
  const enfant = Bun.spawn(commande, {
    cwd: RACINE,
    env: { ...process.env, SITE_ORIGINE: origine },
    stdout: "pipe",
    stderr: "pipe",
  });
  const [sortie, erreur] = await Promise.all([
    new Response(enfant.stdout).text(),
    new Response(enfant.stderr).text(),
  ]);
  return { code: await enfant.exited, texte: `${sortie}\n${erreur}` };
}

const REACT = ["bun", "run", "--filter", "@weave/web", "build"] as const;
const YEW = [
  "cargo",
  "run",
  "--quiet",
  "--manifest-path",
  "apps/site/Cargo.toml",
  "--features",
  "ssr",
  "--bin",
  "rendre",
] as const;

let fautes = 0;

for (const cas of CAS) {
  console.log(`\n  SITE_ORIGINE=${cas.origine}`);
  console.log(`    (${cas.pourquoi})`);

  const [react, yew] = await Promise.all([lancer(REACT, cas.origine), lancer(YEW, cas.origine)]);

  for (const [quoi, resultat] of [
    ["React", react],
    ["Yew", yew],
  ] as const) {
    if (resultat.code === 0) {
      fautes += 1;
      console.log(`    ✗ ${quoi} a CONSTRUIT au lieu de refuser`);
      continue;
    }
    if (!resultat.texte.includes(cas.attendu)) {
      fautes += 1;
      console.log(`    ✗ ${quoi} refuse, mais sans dire « ${cas.attendu} »`);
      // Les premières lignes suffisent à comprendre : un refus écrit beaucoup.
      for (const ligne of resultat.texte.split("\n").filter(Boolean).slice(0, 4)) {
        console.log(`        ${ligne.trim()}`);
      }
      continue;
    }
    console.log(`    ✓ ${quoi} refuse, en disant « ${cas.attendu} »`);
  }
}

/*
 * Et sans origine, les deux doivent CONSTRUIRE.
 *
 * Une garde qui refuse tout est inutile autant qu'une garde qui n'refuse rien :
 * le site doit rester constructible en local tant que la société n'est pas
 * immatriculée, sans quoi on finirait par poser des valeurs bidon pour
 * avancer — exactement ce que ces refus existent pour empêcher.
 *
 * Ce cas remet aussi `dist` et `dist-rs` dans l'état que `comparer.ts` attend :
 * les constructions ci-dessus les ont laissés avec une origine fausse.
 */
console.log("\n  Sans SITE_ORIGINE");
for (const [quoi, commande] of [
  ["React", REACT],
  ["Yew", YEW],
] as const) {
  const resultat = await lancer(commande, "");
  if (resultat.code !== 0) {
    fautes += 1;
    console.log(`    ✗ ${quoi} a REFUSÉ alors que rien n'est posé`);
    for (const ligne of resultat.texte.split("\n").filter(Boolean).slice(-4)) {
      console.log(`        ${ligne.trim()}`);
    }
  } else {
    console.log(`    ✓ ${quoi} construit`);
  }
}

console.log(`\n  ${CAS.length} refus attendus de chaque côté, ${fautes} désaccord(s).`);
if (fautes > 0) process.exit(1);
