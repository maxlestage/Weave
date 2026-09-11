/**
 * Jeu de données de développement.
 *
 * Une petite population de jeunes adultes dans six villes, et surtout des plans
 * pour les jours qui viennent : sans plans, le fil est vide et il n'y a rien à
 * regarder. Idempotent : on peut le relancer sans dupliquer.
 */
import { PLAN_CAPACITY_GROUP_MAX, PLAN_CATEGORIES, type PlanCategory } from "@weave/contracts";
import { emailHash } from "./lib/crypto.ts";
import { log } from "./lib/log.ts";
import { prisma } from "./lib/prisma.ts";

const CITIES: Record<string, { lat: number; lon: number }> = {
  Paris: { lat: 48.86, lon: 2.35 },
  Lyon: { lat: 45.76, lon: 4.84 },
  Marseille: { lat: 43.3, lon: 5.37 },
  Bordeaux: { lat: 44.84, lon: -0.58 },
  Nantes: { lat: 47.22, lon: -1.55 },
  Lille: { lat: 50.63, lon: 3.06 },
};

type Ville = keyof typeof CITIES;

const PEOPLE: { name: string; gender: string; city: Ville; year: number }[] = [
  // Paris — assez nombreux pour qu'un fil parisien ait de la matière.
  { name: "Camille", gender: "femme", city: "Paris", year: 1999 },
  { name: "Inès", gender: "femme", city: "Paris", year: 2000 },
  { name: "Sofia", gender: "femme", city: "Paris", year: 2001 },
  { name: "Louise", gender: "femme", city: "Paris", year: 2002 },
  { name: "Anouk", gender: "femme", city: "Paris", year: 2003 },
  { name: "Salomé", gender: "femme", city: "Paris", year: 2004 },
  { name: "Nour", gender: "femme", city: "Paris", year: 2005 },
  { name: "Agathe", gender: "femme", city: "Paris", year: 2006 },
  { name: "Elsa", gender: "femme", city: "Paris", year: 1999 },
  { name: "Margaux", gender: "femme", city: "Paris", year: 2000 },
  { name: "Jonas", gender: "homme", city: "Paris", year: 2001 },
  { name: "Théo", gender: "homme", city: "Paris", year: 2002 },
  { name: "Aurélien", gender: "homme", city: "Paris", year: 2003 },
  { name: "Noé", gender: "homme", city: "Paris", year: 2004 },
  { name: "Hugo", gender: "homme", city: "Paris", year: 2005 },
  { name: "Ismaël", gender: "homme", city: "Paris", year: 2006 },
  { name: "Victor", gender: "homme", city: "Paris", year: 1999 },
  { name: "Antoine", gender: "homme", city: "Paris", year: 2000 },
  { name: "Gaspard", gender: "homme", city: "Paris", year: 2001 },
  { name: "Simon", gender: "homme", city: "Paris", year: 2002 },
  { name: "Alex", gender: "non_binaire", city: "Paris", year: 2003 },
  { name: "Charlie", gender: "non_binaire", city: "Paris", year: 2004 },
  { name: "Camille B", gender: "non_binaire", city: "Paris", year: 2005 },

  // Lyon
  { name: "Léa", gender: "femme", city: "Lyon", year: 2000 },
  { name: "Manon", gender: "femme", city: "Lyon", year: 2001 },
  { name: "Clara", gender: "femme", city: "Lyon", year: 2002 },
  { name: "Ravi", gender: "homme", city: "Lyon", year: 2003 },
  { name: "Paul", gender: "homme", city: "Lyon", year: 2004 },
  { name: "Youssef", gender: "homme", city: "Lyon", year: 2005 },
  { name: "Sacha", gender: "non_binaire", city: "Lyon", year: 2006 },

  // Marseille
  { name: "Malik", gender: "homme", city: "Marseille", year: 2000 },
  { name: "Lucas", gender: "homme", city: "Marseille", year: 2001 },
  { name: "Nina", gender: "femme", city: "Marseille", year: 2002 },
  { name: "Jade", gender: "femme", city: "Marseille", year: 2003 },

  // Bordeaux
  { name: "Mathilde", gender: "femme", city: "Bordeaux", year: 2004 },
  { name: "Romain", gender: "homme", city: "Bordeaux", year: 2005 },
  { name: "Chloé", gender: "femme", city: "Bordeaux", year: 2000 },

  // Nantes
  { name: "Basile", gender: "homme", city: "Nantes", year: 2001 },
  { name: "Maëlle", gender: "femme", city: "Nantes", year: 2002 },

  // Lille
  { name: "Adrien", gender: "homme", city: "Lille", year: 2003 },
  { name: "Zoé", gender: "femme", city: "Lille", year: 2004 },
];

const BIOS = [
  "Toujours partante pour un truc décidé la veille.",
  "Je cuisine trop pour une personne, d'où les invitations.",
  "Je connais mal ma ville, je compte sur vous.",
  "Deux vitesses : rien, ou tout le week-end dehors.",
  "Je viens d'emménager, je repars de zéro côté bande.",
  "Je préfère marcher deux heures que prendre le métro.",
  "Je dis oui d'abord, je regarde l'heure ensuite.",
  "En stage la semaine, disponible dès vendredi soir.",
];

/**
 * Des plans écrits comme on les écrirait : un titre qui dit ce qu'on fait, une
 * note qui dit pourquoi et comment. Jamais une annonce, jamais un profil.
 */
const PLANS: { title: string; note: string; category: PlanCategory; capacity: number }[] = [
  {
    title: "Marché de la Croix-Rousse puis brunch",
    note: "Je fais les courses de la semaine et je traîne. Venez si vous aimez goûter dix choses avant d'acheter.",
    category: "repas",
    capacity: 2,
  },
  {
    title: "Bloc au mur de 19 h, niveau débutant",
    note: "Je grimpe depuis six mois, très mal. On peut y aller ensemble, l'entrée est à 12 €.",
    category: "sport",
    capacity: 1,
  },
  {
    title: "Expo photo, puis un verre pour en dire du mal",
    note: "J'ai un billet de trop. La moitié sera nulle, c'est tout l'intérêt.",
    category: "culture",
    capacity: 1,
  },
  {
    title: "Concert d'un groupe que personne ne connaît",
    note: "Petite salle, 8 € à l'entrée. Je n'ai écouté que deux morceaux et j'ai aimé.",
    category: "musique",
    capacity: 2,
  },
  {
    title: "Soirée jeux de société chez des amis",
    note: "On est quatre, il manque quelqu'un. Rien de compétitif, promis.",
    category: "jeux",
    capacity: 1,
  },
  {
    title: "Balade au bord de l'eau, dix kilomètres",
    note: "Départ tranquille, pause sandwich au milieu. Prévoir des chaussures correctes.",
    category: "balade",
    capacity: 3,
  },
  {
    title: "Coup de main à la distribution alimentaire",
    note: "Deux heures le samedi matin. On termine par un café, c'est le meilleur moment.",
    category: "benevolat",
    capacity: 2,
  },
  {
    title: "Ciné en VO, film de trois heures",
    note: "Personne ne veut venir avec moi, je comprends. Séance de 20 h 15.",
    category: "culture",
    capacity: 1,
  },
  {
    title: "Course tranquille au parc, 5 km",
    note: "Allure conversation. Je cherche quelqu'un pour ne pas annuler à la dernière minute.",
    category: "sport",
    capacity: 2,
  },
  {
    title: "Friperies puis café, l'après-midi entier",
    note: "Trois adresses repérées. Budget serré, patience requise.",
    category: "sortie",
    capacity: 1,
  },
  {
    title: "Karaoké, et j'assume mon répertoire",
    note: "Variété française exclusivement. On peut être quatre.",
    category: "musique",
    capacity: 3,
  },
  {
    title: "Je teste une recette coréenne, venez goûter",
    note: "Première fois que je fais ça. Il y aura du riz en secours.",
    category: "repas",
    capacity: 2,
  },
];

function slug(value: string): string {
  return value
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase()
    .replace(/[^a-z0-9]/g, "");
}

/**
 * Décalage déterministe autour d'une ville, pour que les distances ne soient
 * pas toutes nulles. On reste dans l'ordre du kilomètre — comme en production,
 * où les coordonnées sont arrondies avant d'être écrites.
 */
function autour(base: number, index: number, pas: number): number {
  return Math.round((base + ((index % 7) - 3) * pas) * 100) / 100;
}

async function seedPeople(): Promise<{ id: string; city: Ville; index: number }[]> {
  const comptes: { id: string; city: Ville; index: number }[] = [];

  for (const [index, personne] of PEOPLE.entries()) {
    const handle = `${slug(personne.name)}${index}`;
    const email = `${handle}@weave.test`;
    const ville = CITIES[personne.city]!;

    const compte = await prisma.account.upsert({
      where: { email },
      create: {
        email,
        emailHash: emailHash(email),
        handle,
        displayName: personne.name,
        birthDate: new Date(Date.UTC(personne.year, (index % 12) + 1, ((index * 3) % 27) + 1)),
        status: "active",
        verified: index % 4 === 0,
        preference: { create: {} },
        subscription: { create: { tier: "depart" } },
      },
      update: { status: "active" },
      select: { id: true },
    });

    await prisma.profile.upsert({
      where: { accountId: compte.id },
      create: {
        accountId: compte.id,
        city: personne.city,
        latRounded: autour(ville.lat, index, 0.03),
        lonRounded: autour(ville.lon, index, 0.04),
        gender: personne.gender,
        bio: BIOS[index % BIOS.length]!,
      },
      update: {},
    });

    comptes.push({ id: compte.id, city: personne.city, index });
  }

  return comptes;
}

/**
 * Publie des plans étalés sur la semaine à venir. Le seed en recrée un jeu
 * complet à chaque exécution : des plans datés d'hier ne servent à rien, et
 * les laisser fausserait la lecture du fil.
 */
async function seedPlans(comptes: { id: string; city: Ville; index: number }[]): Promise<number> {
  await prisma.plan.deleteMany({ where: { authorId: { in: comptes.map((c) => c.id) } } });

  const maintenant = Date.now();
  let publies = 0;

  for (const compte of comptes) {
    // Deux plans sur trois personnes : tout le monde ne publie pas, et un fil
    // où chacun propose quelque chose ne ressemblerait à rien de réel.
    const combien = compte.index % 3 === 2 ? 0 : 1 + (compte.index % 2);

    for (let n = 0; n < combien; n++) {
      const modele = PLANS[(compte.index * 5 + n * 3) % PLANS.length]!;
      const dansHeures = 6 + ((compte.index * 11 + n * 29) % 160);
      const ville = CITIES[compte.city]!;

      await prisma.plan.create({
        data: {
          authorId: compte.id,
          title: modele.title,
          note: modele.note,
          category: modele.category,
          startsAt: new Date(maintenant + dansHeures * 60 * 60 * 1000),
          city: compte.city,
          latRounded: autour(ville.lat, compte.index, 0.03),
          lonRounded: autour(ville.lon, compte.index, 0.04),
          capacity: Math.min(PLAN_CAPACITY_GROUP_MAX, modele.capacity),
        },
      });
      publies += 1;
    }
  }

  return publies;
}

async function main(): Promise<void> {
  log.info("Semis en cours…");

  const comptes = await seedPeople();
  const plans = await seedPlans(comptes);

  log.info("Semis terminé", {
    comptes: comptes.length,
    plans,
    categories: PLAN_CATEGORIES.length,
  });
}

await main();
await prisma.$disconnect();
