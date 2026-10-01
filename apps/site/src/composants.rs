//! Briques d'interface partagées par les sections du site.

use yew::prelude::*;

/// Les six fils de la palette, adressables par numéro.
///
/// Un type, là où le TypeScript avait `1 | 2 | 3 | 4 | 5 | 6`. Même intention,
/// même garantie : `filVar(7)` ne compilait pas, `Fil::try_from(7)` n'existe
/// pas.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Fil {
    Un,
    Deux,
    Trois,
    Quatre,
    Cinq,
    Six,
}

impl Fil {
    pub const fn var(self) -> &'static str {
        match self {
            Fil::Un => "var(--fil-1)",
            Fil::Deux => "var(--fil-2)",
            Fil::Trois => "var(--fil-3)",
            Fil::Quatre => "var(--fil-4)",
            Fil::Cinq => "var(--fil-5)",
            Fil::Six => "var(--fil-6)",
        }
    }

    /// Aplat très pâle d'un fil, utilisable derrière du texte courant.
    pub fn teinte(self, pourcentage: u8) -> String {
        format!(
            "color-mix(in oklab, {} {pourcentage}%, var(--fond))",
            self.var()
        )
    }
}

#[derive(Properties, PartialEq)]
pub struct ProprietesSection {
    pub id: AttrValue,
    pub titre: AttrValue,
    #[prop_or_default]
    pub chapeau: Option<AttrValue>,
    /// Couleur de la section : chacune a la sienne.
    pub fil: Fil,
    #[prop_or(false)]
    pub alterne: bool,
    #[prop_or_default]
    pub children: Html,
}

#[function_component]
pub fn Section(p: &ProprietesSection) -> Html {
    // `background: …`, et non la seule valeur.
    //
    // Ma première version posait `style="color-mix(…)"` — une déclaration sans
    // nom de propriété, que le navigateur ignore en silence. Les sections
    // alternées auraient perdu leur fond sans qu'aucun test ne s'en plaigne :
    // le texte, lui, était intact. C'est le genre de régression qu'un contrôle
    // sur le contenu ne voit pas, et qui se remarque à l'œil.
    let fond = p
        .alterne
        .then(|| format!("background: {}", p.fil.teinte(7)));
    html! {
        <section
            id={p.id.clone()}
            aria-labelledby={format!("{}-titre", p.id)}
            class="px-5 py-16 sm:px-8 sm:py-24"
            style={fond}
        >
            <div class="mx-auto w-full max-w-5xl">
                // Un court trait de couleur annonce la section.
                <span
                    aria-hidden="true"
                    class="mb-5 block h-1.5 w-16 rounded-full"
                    style={format!("background: {}", p.fil.var())}
                />
                <h2
                    id={format!("{}-titre", p.id)}
                    class="text-3xl leading-tight font-semibold tracking-tight sm:text-4xl"
                    style="font-family: var(--font-titre)"
                >
                    { p.titre.clone() }
                </h2>
                if let Some(chapeau) = &p.chapeau {
                    <p
                        class="mt-4 max-w-2xl text-lg leading-relaxed"
                        style="color: var(--texte-doux)"
                    >
                        { chapeau.clone() }
                    </p>
                }
                <div class="mt-10">{ p.children.clone() }</div>
            </div>
        </section>
    }
}

#[derive(Properties, PartialEq)]
pub struct ProprietesCarte {
    pub fil: Fil,
    #[prop_or(false)]
    pub accentuee: bool,
    #[prop_or_default]
    pub children: Html,
}

#[function_component]
pub fn Carte(p: &ProprietesCarte) -> Html {
    let bordure = if p.accentuee {
        p.fil.var()
    } else {
        "var(--bordure)"
    };
    let mut style = format!("background: var(--carte); border: 2px solid {bordure}");
    if p.accentuee {
        style.push_str(&format!(
            "; box-shadow: 0 0 0 5px color-mix(in oklab, {} 16%, transparent)",
            p.fil.var()
        ));
    }
    html! {
        <div class="rounded-3xl p-6 sm:p-7" {style}>{ p.children.clone() }</div>
    }
}

#[derive(Properties, PartialEq)]
pub struct ProprietesFil {
    pub fil: Fil,
    #[prop_or_default]
    pub children: Html,
}

#[function_component]
pub fn Etiquette(p: &ProprietesFil) -> Html {
    html! {
        <span
            class="inline-block rounded-full px-3.5 py-1.5 text-xs font-bold tracking-wide uppercase"
            style={format!("background: {}; color: var(--sur-accent)", p.fil.var())}
        >
            { p.children.clone() }
        </span>
    }
}

/// Pastille numérotée, pour les listes ordonnées.
#[function_component]
pub fn Pastille(p: &ProprietesFil) -> Html {
    html! {
        <span
            class="flex h-11 w-11 shrink-0 items-center justify-center rounded-2xl text-base font-bold tabular-nums"
            style={format!("background: {}; color: var(--sur-accent)", p.fil.var())}
            aria-hidden="true"
        >
            { p.children.clone() }
        </span>
    }
}
