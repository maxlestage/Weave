//! Les langues du site.
//!
//! Le français reste la langue de référence : c'est celle dans laquelle les
//! textes sont écrits et relus, et celle qui fait foi pour les pages
//! juridiques. Les autres en sont des traductions.

/// Une langue du site.
///
/// Une énumération, là où le TypeScript avait une union de chaînes. Le gain
/// n'est pas cosmétique : un `match` sur `Langue` est exhaustif, et ajouter
/// une quatrième langue fait échouer la compilation à CHAQUE endroit qui doit
/// la traduire. En TypeScript, un `Record<Langue, T>` oublié se voyait au
/// typage ; un `match` incomplet, lui, ne compile pas du tout.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Langue {
    Fr,
    En,
    Es,
}

pub const LANGUES: [Langue; 3] = [Langue::Fr, Langue::En, Langue::Es];
pub const LANGUE_PAR_DEFAUT: Langue = Langue::Fr;

impl Langue {
    /// Le code court, tel qu'il apparaît dans l'adresse et dans `lang`.
    pub const fn code(self) -> &'static str {
        match self {
            Langue::Fr => "fr",
            Langue::En => "en",
            Langue::Es => "es",
        }
    }

    /// Le nom de la langue, DANS cette langue : c'est ainsi qu'on la propose.
    pub const fn nom(self) -> &'static str {
        match self {
            Langue::Fr => "Français",
            Langue::En => "English",
            Langue::Es => "Español",
        }
    }

    /// L'étiquette de locale pour `og:locale`.
    pub const fn locale(self) -> &'static str {
        match self {
            Langue::Fr => "fr_FR",
            Langue::En => "en_US",
            Langue::Es => "es_ES",
        }
    }

    /// Le préfixe d'adresse. Le français n'en a pas : il occupe la racine, et
    /// lui en donner un casserait les adresses déjà publiées.
    pub fn prefixe(self) -> String {
        if self == LANGUE_PAR_DEFAUT {
            String::new()
        } else {
            format!("/{}", self.code())
        }
    }

    /// La langue que désigne une adresse.
    ///
    /// Lue au démarrage côté client pour hydrater dans la MÊME langue que
    /// celle du rendu : une page rendue en anglais qui s'hydraterait en
    /// français remplacerait tout son texte au premier affichage.
    pub fn du_chemin(chemin: &str) -> Langue {
        let premier = chemin.split('/').find(|s| !s.is_empty()).unwrap_or("");
        LANGUES
            .into_iter()
            .find(|l| l.code() == premier)
            .unwrap_or(LANGUE_PAR_DEFAUT)
    }

    /// L'adresse d'une page dans cette langue.
    pub fn chemin(self, slug: &str) -> String {
        let base = self.prefixe();
        if slug.is_empty() {
            if base.is_empty() {
                "/".to_string()
            } else {
                format!("{base}/")
            }
        } else {
            format!("{base}/{slug}")
        }
    }
}

/// Un contenu décliné dans les trois langues.
///
/// Un tableau plutôt qu'une table associative : l'index est garanti par le
/// type, et il n'y a pas de clé manquante possible. `choisir` ne peut donc
/// pas échouer, là où la version TypeScript indexait un `Record`.
#[derive(Clone, Copy)]
pub struct Traduit<T: 'static>(pub [T; 3]);

impl<T: 'static> Traduit<T> {
    pub fn choisir(&self, langue: Langue) -> &T {
        match langue {
            Langue::Fr => &self.0[0],
            Langue::En => &self.0[1],
            Langue::Es => &self.0[2],
        }
    }
}
