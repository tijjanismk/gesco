//! Formules des leçons pour l'impression (ADR « format des leçons ») : conversion du sous-ensemble
//! LaTeX qu'emploie la bibliothèque vers les mathématiques de Typst.
//!
//! La table est bornée par le relevé de la bibliothèque (une centaine de commandes, `cases`,
//! `array`). Une commande inconnue rend une erreur : l'appelant imprime alors la formule telle
//! qu'elle est écrite, en caractères à chasse fixe, et l'éditeur le signale à l'auteur. La
//! conversion ne refuse jamais d'imprimer.
//!
//! Les symboles sont rendus par leur caractère Unicode, que Typst classe lui-même (relation,
//! opérateur, grand opérateur) : aucun nom de symbole Typst à suivre d'une version à l'autre.

use std::fmt;

/// Imbrication au-delà de laquelle une formule est jugée malformée.
const PROFONDEUR_MAX: usize = 64;

/// Pourquoi une formule ne se convertit pas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormuleNonConvertie {
    /// Commande absente de la table (`\overset`…).
    CommandeInconnue(String),
    /// Environnement absent de la table.
    EnvironnementInconnu(String),
    /// Accolades ou `\left … \right` mal appariés, ou imbrication trop profonde.
    Malformee,
}

impl fmt::Display for FormuleNonConvertie {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CommandeInconnue(c) => write!(f, "commande \\{c} inconnue"),
            Self::EnvironnementInconnu(e) => write!(f, "environnement {e} inconnu"),
            Self::Malformee => f.write_str("formule mal formée"),
        }
    }
}

impl std::error::Error for FormuleNonConvertie {}

/// Convertit une formule LaTeX en mathématiques Typst (le contenu entre `$`).
pub fn latex_vers_typst(latex: &str) -> Result<String, FormuleNonConvertie> {
    let mut c = Convertisseur {
        jetons: jetons(latex),
        position: 0,
        profondeur: 0,
        environnements: Vec::new(),
    };
    let sortie = c.suite(Fin::Texte)?;
    Ok(sortie)
}

/// Un jeton LaTeX.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Jeton {
    /// `\nom` ou `\` suivi d'un caractère qui n'est pas une lettre.
    Commande(String),
    Ouvrante,
    Fermante,
    Exposant,
    Indice,
    Esperluette,
    /// Espace, tabulation ou fin de ligne (insignifiants en mode mathématique).
    Blanc,
    Car(char),
}

fn jetons(latex: &str) -> Vec<Jeton> {
    let mut jetons = Vec::new();
    let mut caracteres = latex.chars().peekable();
    while let Some(c) = caracteres.next() {
        match c {
            '\\' => {
                let mut nom = String::new();
                while let Some(&s) = caracteres.peek() {
                    if s.is_ascii_alphabetic() {
                        nom.push(s);
                        caracteres.next();
                    } else {
                        break;
                    }
                }
                if nom.is_empty() {
                    match caracteres.next() {
                        Some(s) if s.is_whitespace() => jetons.push(Jeton::Commande(" ".into())),
                        Some(s) => jetons.push(Jeton::Commande(s.to_string())),
                        None => jetons.push(Jeton::Commande(String::new())),
                    }
                } else {
                    jetons.push(Jeton::Commande(nom));
                }
            }
            '{' => jetons.push(Jeton::Ouvrante),
            '}' => jetons.push(Jeton::Fermante),
            '^' => jetons.push(Jeton::Exposant),
            '_' => jetons.push(Jeton::Indice),
            '&' => jetons.push(Jeton::Esperluette),
            // Commentaire LaTeX jusqu'à la fin de la ligne.
            '%' => {
                for s in caracteres.by_ref() {
                    if s == '\n' {
                        break;
                    }
                }
                jetons.push(Jeton::Blanc);
            }
            c if c.is_whitespace() => {
                if jetons.last() != Some(&Jeton::Blanc) {
                    jetons.push(Jeton::Blanc);
                }
            }
            c => jetons.push(Jeton::Car(c)),
        }
    }
    jetons
}

/// Ce qui termine une suite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fin {
    /// Fin du texte.
    Texte,
    /// `}`.
    Accolade,
    /// `]` d'un argument facultatif.
    Crochet,
    /// `\right`.
    Droite,
    /// `\end`.
    Environnement,
}

/// Environnement en cours : ce que deviennent `&` et `\\`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Environnement {
    /// `cases` : `&` reste un alignement, `\\` sépare les arguments de `cases(…)`.
    Cas,
    /// `array`, `matrix`… : `&` sépare les cellules, `\\` les lignes de `mat(…)`.
    Matrice,
    /// `aligned` : `&` aligne, `\\` passe à la ligne.
    Aligne,
}

struct Convertisseur {
    jetons: Vec<Jeton>,
    position: usize,
    profondeur: usize,
    environnements: Vec<Environnement>,
}

/// Symbole d'une commande sans argument : son caractère Unicode (ou son équivalent Typst).
fn symbole(nom: &str) -> Option<&'static str> {
    Some(match nom {
        // Lettres grecques.
        "alpha" => "α",
        "beta" => "β",
        "gamma" => "γ",
        "delta" => "δ",
        "epsilon" => "ϵ",
        "varepsilon" => "ε",
        "zeta" => "ζ",
        "eta" => "η",
        "theta" => "θ",
        "vartheta" => "ϑ",
        "iota" => "ι",
        "kappa" => "κ",
        "lambda" => "λ",
        "mu" => "μ",
        "nu" => "ν",
        "xi" => "ξ",
        "pi" => "π",
        "varpi" => "ϖ",
        "rho" => "ρ",
        "varrho" => "ϱ",
        "sigma" => "σ",
        "varsigma" => "ς",
        "tau" => "τ",
        "upsilon" => "υ",
        "phi" => "ϕ",
        "varphi" => "φ",
        "chi" => "χ",
        "psi" => "ψ",
        "omega" => "ω",
        "Gamma" => "Γ",
        "Delta" => "Δ",
        "Theta" => "Θ",
        "Lambda" => "Λ",
        "Xi" => "Ξ",
        "Pi" => "Π",
        "Sigma" => "Σ",
        "Upsilon" => "Υ",
        "Phi" => "Φ",
        "Psi" => "Ψ",
        "Omega" => "Ω",
        // Opérateurs et relations.
        "times" => "×",
        "div" => "÷",
        "circ" => "∘",
        "cdot" => "⋅",
        "pm" => "±",
        "mp" => "∓",
        "ast" => "∗",
        "star" => "⋆",
        "bullet" => "∙",
        "otimes" => "⊗",
        "oplus" => "⊕",
        "wedge" | "land" => "∧",
        "vee" | "lor" => "∨",
        "neg" | "lnot" => "¬",
        "cap" => "∩",
        "cup" => "∪",
        "setminus" => "∖",
        "approx" => "≈",
        "sim" => "∼",
        "simeq" => "≃",
        "cong" => "≅",
        "equiv" => "≡",
        "propto" => "∝",
        "leq" | "le" => "≤",
        "geq" | "ge" => "≥",
        "neq" | "ne" => "≠",
        "ll" => "≪",
        "gg" => "≫",
        "in" => "∈",
        "notin" => "∉",
        "ni" => "∋",
        "subset" => "⊂",
        "subseteq" => "⊆",
        "supset" => "⊃",
        "supseteq" => "⊇",
        "perp" => "⊥",
        "parallel" => "∥",
        "mid" => "∣",
        // Flèches.
        "to" | "rightarrow" => "→",
        "leftarrow" | "gets" => "←",
        "leftrightarrow" => "↔",
        "Rightarrow" => "⇒",
        "Leftarrow" => "⇐",
        "Leftrightarrow" => "⇔",
        "longrightarrow" => "⟶",
        "longleftarrow" => "⟵",
        "Longrightarrow" | "implies" => "⟹",
        "Longleftarrow" => "⟸",
        "Longleftrightarrow" | "iff" => "⟺",
        "mapsto" => "↦",
        "uparrow" => "↑",
        "downarrow" => "↓",
        "rightleftharpoons" => "⇌",
        // Grands opérateurs.
        "sum" => "∑",
        "prod" => "∏",
        "int" => "∫",
        "iint" => "∬",
        "oint" => "∮",
        "bigcup" => "⋃",
        "bigcap" => "⋂",
        // Divers.
        "infty" => "∞",
        "partial" => "∂",
        "nabla" => "∇",
        "forall" => "∀",
        "exists" => "∃",
        "emptyset" | "varnothing" => "∅",
        "angle" => "∠",
        "triangle" => "△",
        "square" => "□",
        "prime" => "′",
        "ell" => "ℓ",
        "hbar" => "ℏ",
        "imath" => "ı",
        "jmath" => "ȷ",
        "ldots" | "dots" | "dotsc" | "dotsb" => "…",
        "cdots" => "⋯",
        "vdots" => "⋮",
        "ddots" => "⋱",
        "degree" => "°",
        // Délimiteurs.
        "lbrace" | "{" => "\\{",
        "rbrace" | "}" => "\\}",
        "langle" => "⟨",
        "rangle" => "⟩",
        "lvert" | "rvert" | "vert" => "\\|",
        "lVert" | "rVert" | "Vert" | "|" => "‖",
        "lfloor" => "⌊",
        "rfloor" => "⌋",
        "lceil" => "⌈",
        "rceil" => "⌉",
        // Caractères échappés.
        "%" => "%",
        "&" => "\\&",
        "#" => "\\#",
        "_" => "\\_",
        "$" => "\\$",
        // Espaces.
        "," => "#h(0.1667em)",
        ":" | ">" => "#h(0.2222em)",
        ";" => "#h(0.2778em)",
        " " => "#h(0.3333em)",
        "!" => "#h(-0.1667em)",
        "quad" => "#h(1em)",
        "qquad" => "#h(2em)",
        _ => return None,
    })
}

/// Opérateurs écrits droits (`\sin x`) ; vrai pour ceux qui portent leurs bornes dessous.
fn operateur(nom: &str) -> Option<bool> {
    match nom {
        "sin" | "cos" | "tan" | "cot" | "sec" | "csc" | "arcsin" | "arccos" | "arctan" | "sinh"
        | "cosh" | "tanh" | "ln" | "log" | "exp" | "arg" | "deg" | "dim" | "ker" | "hom" | "Pr" => {
            Some(false)
        }
        "lim" | "max" | "min" | "sup" | "inf" | "det" | "gcd" | "limsup" | "liminf" => Some(true),
        _ => None,
    }
}

/// Commandes sans effet à l'impression.
fn sans_effet(nom: &str) -> bool {
    matches!(
        nom,
        "displaystyle"
            | "textstyle"
            | "scriptstyle"
            | "limits"
            | "nolimits"
            | "big"
            | "Big"
            | "bigg"
            | "Bigg"
            | "bigl"
            | "bigr"
            | "Bigl"
            | "Bigr"
            | "biggl"
            | "biggr"
            | "Biggl"
            | "Biggr"
            | "hline"
            | "nonumber"
            | "notag"
    )
}

/// Caractère ordinaire d'une formule, en Typst. Parenthèses, crochets, virgules, barres et
/// chevrons sont échappés : ils ne forment ni groupe, ni argument, ni étiquette, comme en LaTeX ;
/// le trait d'union devient le signe moins, ce qui évite les raccourcis de Typst (`->`).
fn caractere(c: char) -> String {
    match c {
        '(' | ')' | '[' | ']' | ',' | ';' | '/' | '"' | '#' | '$' | '@' | '`' | '|' | '\\'
        | '<' | '>' => format!("\\{c}"),
        '-' => "−".to_owned(),
        '*' => "∗".to_owned(),
        '~' => "#h(0.3333em)".to_owned(),
        autre => autre.to_string(),
    }
}

/// Texte d'une formule placé entre guillemets Typst.
fn chaine(texte: &str) -> String {
    let mut sortie = String::from("\"");
    for c in texte.chars() {
        if matches!(c, '"' | '\\') {
            sortie.push('\\');
        }
        sortie.push(c);
    }
    sortie.push('"');
    sortie
}

/// Vrai si un morceau commence ou finit par un caractère d'identifiant Typst : deux tels morceaux
/// accolés formeraient un seul nom (`ab`, `x2`), il faut une espace entre eux.
fn bord_identifiant(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '.'
}

/// Ajoute un morceau à la sortie, avec une espace si les deux bords formeraient un nom.
fn ajouter(sortie: &mut String, morceau: &str) {
    if morceau.is_empty() {
        return;
    }
    let premier = morceau.chars().next().unwrap_or(' ');
    if let Some(dernier) = sortie.chars().last() {
        let nombre = dernier.is_ascii_digit() && (premier.is_ascii_digit() || premier == '.')
            || dernier == '.' && premier.is_ascii_digit();
        let accroche = matches!(premier, '^' | '_' | '\'');
        // `2x` : un chiffre suivi d'une seule lettre reste accolé, comme en LaTeX.
        let coefficient = dernier.is_ascii_digit() && morceau.chars().count() == 1;
        if !nombre
            && !accroche
            && !coefficient
            && (bord_identifiant(dernier) && bord_identifiant(premier)
                || dernier == ')' && premier == '(')
        {
            sortie.push(' ');
        }
    }
    sortie.push_str(morceau);
}

impl Convertisseur {
    fn suivant(&mut self) -> Option<Jeton> {
        let j = self.jetons.get(self.position).cloned();
        if j.is_some() {
            self.position += 1;
        }
        j
    }

    fn regarder(&self) -> Option<&Jeton> {
        self.jetons.get(self.position)
    }

    fn sauter_blancs(&mut self) {
        while self.regarder() == Some(&Jeton::Blanc) {
            self.position += 1;
        }
    }

    fn entrer(&mut self) -> Result<(), FormuleNonConvertie> {
        self.profondeur += 1;
        if self.profondeur > PROFONDEUR_MAX {
            return Err(FormuleNonConvertie::Malformee);
        }
        Ok(())
    }

    /// Convertit une suite jusqu'à sa fin.
    fn suite(&mut self, fin: Fin) -> Result<String, FormuleNonConvertie> {
        self.entrer()?;
        let mut sortie = String::new();
        loop {
            let Some(jeton) = self.suivant() else {
                if fin == Fin::Texte {
                    break;
                }
                return Err(FormuleNonConvertie::Malformee);
            };
            match jeton {
                Jeton::Fermante if fin == Fin::Accolade => break,
                Jeton::Fermante => return Err(FormuleNonConvertie::Malformee),
                Jeton::Car(']') if fin == Fin::Crochet => break,
                Jeton::Commande(nom) if nom == "right" => {
                    if fin == Fin::Droite {
                        break;
                    }
                    return Err(FormuleNonConvertie::Malformee);
                }
                Jeton::Commande(nom) if nom == "end" => {
                    if fin == Fin::Environnement {
                        break;
                    }
                    return Err(FormuleNonConvertie::Malformee);
                }
                Jeton::Blanc => {}
                Jeton::Exposant | Jeton::Indice => {
                    let signe = if jeton == Jeton::Exposant { "^" } else { "_" };
                    let argument = self.argument()?;
                    let sans_base = sortie
                        .trim_end()
                        .chars()
                        .last()
                        .is_none_or(|c| matches!(c, '(' | ',' | '&' | ';' | '\\'));
                    if sans_base {
                        ajouter(&mut sortie, "\"\"");
                    }
                    ajouter(&mut sortie, &format!("{signe}({argument})"));
                }
                Jeton::Esperluette => {
                    let morceau = match self.environnements.last() {
                        Some(Environnement::Matrice) => ",",
                        _ => "&",
                    };
                    sortie.push_str(morceau);
                }
                Jeton::Car(c) if c.is_ascii_digit() => {
                    let mut nombre = c.to_string();
                    while let Some(Jeton::Car(s)) = self.regarder() {
                        let s = *s;
                        let point_puis_chiffre = s == '.'
                            && matches!(
                                self.jetons.get(self.position + 1),
                                Some(Jeton::Car(d)) if d.is_ascii_digit()
                            );
                        if s.is_ascii_digit() || point_puis_chiffre {
                            nombre.push(s);
                            self.position += 1;
                        } else {
                            break;
                        }
                    }
                    ajouter(&mut sortie, &nombre);
                }
                _ => {
                    self.position -= 1;
                    let morceau = self.atome()?;
                    ajouter(&mut sortie, &morceau);
                }
            }
        }
        self.profondeur -= 1;
        Ok(sortie)
    }

    /// Argument d'une commande ou d'un exposant : un groupe `{…}` ou un seul atome.
    fn argument(&mut self) -> Result<String, FormuleNonConvertie> {
        self.sauter_blancs();
        match self.regarder() {
            Some(Jeton::Ouvrante) => {
                self.position += 1;
                self.suite(Fin::Accolade)
            }
            Some(Jeton::Car(c)) if c.is_ascii_digit() => {
                // `x^23` : seul le premier chiffre est l'exposant, comme en LaTeX.
                let c = *c;
                self.position += 1;
                Ok(c.to_string())
            }
            Some(_) => self.atome(),
            None => Err(FormuleNonConvertie::Malformee),
        }
    }

    /// Texte brut d'un groupe `{…}` (pour `\text`, `\operatorname`).
    fn texte(&mut self) -> Result<String, FormuleNonConvertie> {
        self.sauter_blancs();
        if self.regarder() != Some(&Jeton::Ouvrante) {
            // `\text x` : un seul caractère.
            return match self.suivant() {
                Some(Jeton::Car(c)) => Ok(c.to_string()),
                _ => Err(FormuleNonConvertie::Malformee),
            };
        }
        self.position += 1;
        let mut texte = String::new();
        let mut ouvertes = 0_usize;
        loop {
            match self.suivant() {
                None => return Err(FormuleNonConvertie::Malformee),
                Some(Jeton::Ouvrante) => ouvertes += 1,
                Some(Jeton::Fermante) => {
                    if ouvertes == 0 {
                        break;
                    }
                    ouvertes -= 1;
                }
                Some(Jeton::Car(c)) => texte.push(c),
                Some(Jeton::Blanc) => texte.push(' '),
                Some(Jeton::Exposant) => texte.push('^'),
                Some(Jeton::Indice) => texte.push('_'),
                Some(Jeton::Esperluette) => texte.push('&'),
                Some(Jeton::Commande(nom)) => match nom.as_str() {
                    "%" | "&" | "#" | "_" | "$" | "{" | "}" => texte.push_str(&nom),
                    "," | ";" | ":" | " " | "quad" | "qquad" | "!" => texte.push(' '),
                    autre => return Err(FormuleNonConvertie::CommandeInconnue(autre.to_owned())),
                },
            }
        }
        Ok(texte)
    }

    /// Délimiteur après `\left` ou `\right` ; `.` n'en met aucun.
    fn delimiteur(&mut self) -> Result<String, FormuleNonConvertie> {
        self.sauter_blancs();
        match self.suivant() {
            Some(Jeton::Car('.')) => Ok(String::new()),
            Some(Jeton::Car(c)) => Ok(caractere(c)),
            Some(Jeton::Commande(nom)) => symbole(&nom)
                .map(str::to_owned)
                .ok_or(FormuleNonConvertie::CommandeInconnue(nom)),
            _ => Err(FormuleNonConvertie::Malformee),
        }
    }

    /// Un atome : caractère, groupe ou commande avec ses arguments.
    fn atome(&mut self) -> Result<String, FormuleNonConvertie> {
        self.sauter_blancs();
        match self.suivant() {
            None => Err(FormuleNonConvertie::Malformee),
            Some(Jeton::Ouvrante) => {
                let contenu = self.suite(Fin::Accolade)?;
                // `{,}` : virgule décimale, sans l'espace d'une ponctuation.
                if contenu == "\\," {
                    return Ok("class(\"normal\", \\,)".to_owned());
                }
                Ok(if contenu.is_empty() {
                    "\"\"".to_owned()
                } else {
                    format!("({contenu})")
                })
            }
            Some(Jeton::Car(c)) => Ok(caractere(c)),
            Some(Jeton::Commande(nom)) => self.commande(&nom),
            Some(Jeton::Fermante | Jeton::Exposant | Jeton::Indice | Jeton::Esperluette) => {
                Err(FormuleNonConvertie::Malformee)
            }
            Some(Jeton::Blanc) => Ok(String::new()),
        }
    }

    fn commande(&mut self, nom: &str) -> Result<String, FormuleNonConvertie> {
        if let Some(s) = symbole(nom) {
            return Ok(s.to_owned());
        }
        if let Some(bornes_dessous) = operateur(nom) {
            return Ok(if bornes_dessous {
                format!("op({}, limits: #true)", chaine(nom))
            } else {
                format!("op({})", chaine(nom))
            });
        }
        if sans_effet(nom) {
            return Ok(String::new());
        }
        let un = |c: &mut Self, f: &str| -> Result<String, FormuleNonConvertie> {
            let a = c.argument()?;
            Ok(format!("{f}({a})"))
        };
        match nom {
            "\\" => Ok(match self.environnements.last() {
                Some(Environnement::Cas) => ",".to_owned(),
                Some(Environnement::Matrice) => ";".to_owned(),
                Some(Environnement::Aligne) | None => " \\ ".to_owned(),
            }),
            "frac" | "dfrac" | "tfrac" | "cfrac" => {
                let haut = self.argument()?;
                let bas = self.argument()?;
                let fraction = format!("frac({haut}, {bas})");
                Ok(match nom {
                    "dfrac" | "cfrac" => format!("display({fraction})"),
                    "tfrac" => format!("inline({fraction})"),
                    _ => fraction,
                })
            }
            "binom" | "dbinom" => {
                let haut = self.argument()?;
                let bas = self.argument()?;
                Ok(format!("binom({haut}, {bas})"))
            }
            "sqrt" => {
                self.sauter_blancs();
                if self.regarder() == Some(&Jeton::Car('[')) {
                    self.position += 1;
                    let indice = self.suite(Fin::Crochet)?;
                    let radicande = self.argument()?;
                    Ok(format!("root({indice}, {radicande})"))
                } else {
                    un(self, "sqrt")
                }
            }
            "vec" | "overrightarrow" => un(self, "arrow"),
            "overleftarrow" => un(self, "arrow.l"),
            "hat" | "widehat" => un(self, "hat"),
            "tilde" | "widetilde" => un(self, "tilde"),
            "bar" => un(self, "macron"),
            "overline" => un(self, "overline"),
            "underline" => un(self, "underline"),
            "dot" => un(self, "dot"),
            "ddot" => un(self, "dot.double"),
            "mathbb" => un(self, "bb"),
            "mathcal" => un(self, "cal"),
            "mathbf" | "boldsymbol" | "bm" => un(self, "bold"),
            "mathit" => un(self, "italic"),
            "mathrm" => un(self, "upright"),
            "text" | "textrm" | "mbox" | "textnormal" => {
                let t = self.texte()?;
                Ok(format!("upright({})", chaine(&t)))
            }
            "textbf" => {
                let t = self.texte()?;
                Ok(format!("bold(upright({}))", chaine(&t)))
            }
            "textit" | "emph" => {
                let t = self.texte()?;
                Ok(chaine(&t))
            }
            "operatorname" => {
                let t = self.texte()?;
                Ok(format!("op({})", chaine(t.trim())))
            }
            "left" => {
                let ouvrant = self.delimiteur()?;
                let contenu = self.suite(Fin::Droite)?;
                let fermant = self.delimiteur()?;
                if ouvrant.is_empty() && fermant.is_empty() {
                    Ok(format!("({contenu})"))
                } else if ouvrant.is_empty() || fermant.is_empty() {
                    Ok(format!("{ouvrant} {contenu} {fermant}"))
                } else {
                    Ok(format!("lr({ouvrant} {contenu} {fermant})"))
                }
            }
            "begin" => self.environnement(),
            autre => Err(FormuleNonConvertie::CommandeInconnue(autre.to_owned())),
        }
    }

    fn environnement(&mut self) -> Result<String, FormuleNonConvertie> {
        let nom = self.texte()?;
        let (genre, debut, fin) = match nom.as_str() {
            "cases" => (Environnement::Cas, "cases(", ")"),
            "array" => {
                // Spécification des colonnes (`{ll}`), sans effet.
                let _ = self.texte()?;
                (Environnement::Matrice, "mat(delim: #none, ", ")")
            }
            "matrix" => (Environnement::Matrice, "mat(delim: #none, ", ")"),
            "pmatrix" => (Environnement::Matrice, "mat(delim: \"(\", ", ")"),
            "bmatrix" => (Environnement::Matrice, "mat(delim: \"[\", ", ")"),
            "vmatrix" => (Environnement::Matrice, "mat(delim: \"|\", ", ")"),
            "aligned" | "align" | "align*" | "gathered" => (Environnement::Aligne, "", ""),
            autre => return Err(FormuleNonConvertie::EnvironnementInconnu(autre.to_owned())),
        };
        self.environnements.push(genre);
        let contenu = self.suite(Fin::Environnement);
        self.environnements.pop();
        let contenu = contenu?;
        let fin_nom = self.texte()?;
        if fin_nom != nom {
            return Err(FormuleNonConvertie::Malformee);
        }
        // Une ligne vide finale (`… \\ \end{cases}`) ne fait pas d'argument vide.
        let contenu = contenu.trim().trim_end_matches([',', ';']).to_owned();
        Ok(format!("{debut}{contenu}{fin}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(latex: &str) -> String {
        latex_vers_typst(latex).unwrap_or_else(|e| format!("ERREUR {e}"))
    }

    #[test]
    fn lettres_chiffres_et_symboles() {
        assert_eq!(t("2x+3"), "2x+3");
        assert_eq!(t("ab"), "a b");
        assert_eq!(t("x2"), "x 2");
        assert_eq!(t("3.14"), "3.14");
        assert_eq!(t("a \\times b"), "a×b");
        assert_eq!(t("\\alpha\\beta"), "α β");
        assert_eq!(t("3{,}5"), "3 class(\"normal\", \\,)5");
        assert_eq!(t("x<y"), "x\\<y");
    }

    #[test]
    fn fractions_racines_et_scripts() {
        assert_eq!(t("\\frac{1}{2}"), "frac(1, 2)");
        assert_eq!(t("\\dfrac{a}{b}"), "display(frac(a, b))");
        assert_eq!(t("\\sqrt{2}"), "sqrt(2)");
        assert_eq!(t("\\sqrt[3]{x}"), "root(3, x)");
        assert_eq!(t("x^{2}"), "x^(2)");
        assert_eq!(t("x^23"), "x^(2)3");
        assert_eq!(t("{}^{14}C"), "\"\"^(14)C");
        assert_eq!(t("90^\\circ"), "90^(∘)");
    }

    #[test]
    fn parentheses_et_virgules_echappees() {
        assert_eq!(t("f(x, y)"), "f\\(x\\,y\\)");
        assert_eq!(t("]0;1["), "\\]0\\;1\\[");
        assert_eq!(t("a/b"), "a\\/b");
        assert_eq!(t("\\left( \\frac{1}{2} \\right)"), "lr(\\( frac(1, 2) \\))");
        assert_eq!(t("\\left\\{ x \\right."), "\\{ x ");
    }

    #[test]
    fn texte_operateurs_et_environnements() {
        assert_eq!(t("\\text{cm}"), "upright(\"cm\")");
        assert_eq!(t("\\sin x"), "op(\"sin\")x");
        assert_eq!(t("\\lim_{x \\to 0}"), "op(\"lim\", limits: #true)_(x→0)");
        assert_eq!(
            t("\\begin{cases} x & \\text{si } x>0 \\\\ -x & \\text{sinon} \\end{cases}"),
            "cases(x&upright(\"si \")x\\>0,−x&upright(\"sinon\"))"
        );
        assert_eq!(
            t("\\begin{array}{ll} a & b \\\\ c & d \\end{array}"),
            "mat(delim: #none, a,b;c,d)"
        );
    }

    #[test]
    fn commande_inconnue_et_formule_malformee() {
        assert_eq!(
            latex_vers_typst("\\overset{a}{b}"),
            Err(FormuleNonConvertie::CommandeInconnue("overset".to_owned()))
        );
        assert_eq!(
            latex_vers_typst("\\frac{1}{2"),
            Err(FormuleNonConvertie::Malformee)
        );
        assert_eq!(latex_vers_typst("x}"), Err(FormuleNonConvertie::Malformee));
        assert_eq!(
            latex_vers_typst(&"{".repeat(200)),
            Err(FormuleNonConvertie::Malformee)
        );
    }
}
