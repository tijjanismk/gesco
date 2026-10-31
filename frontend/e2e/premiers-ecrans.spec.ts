// Premiers écrans de la vraie application, noyau Rust compris : premier démarrage, phrase de
// secours, connexion, puis chaque section de la barre latérale. Le mode d'essai de `gesco-app`
// donne un espace de données et un dossier WebView2 jetables : l'établissement du poste n'est
// pas touché. Les captures sont écrites dans `e2e/captures/`.
import { type ChildProcess, spawn, spawnSync } from "node:child_process";
import { copyFileSync, existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { type Browser, type Page, chromium, expect, test } from "@playwright/test";

import { catalogue, type Cle } from "../src/i18n/fr-ML";

const EXECUTABLE = join(import.meta.dirname, "..", "..", "target", "debug", "gesco-app.exe");
const CAPTURES = join(import.meta.dirname, "captures");
const PORT_DEBOGAGE = 9223;
const IDENTIFIANT = "direction";
const NOM = "Awa Traoré";
const MOT_DE_PASSE = "essai-premier-demarrage";
const NOUVEAU_MOT_DE_PASSE = "essai-mot-de-passe-change";
const PIN = "2468";
/** Jour où vivent les scénarios (`GESCO_DATE_ESSAI`), quel que soit le jour réel. */
const JOUR_ESSAI = "2026-10-14";

const texte = (cle: Cle) => catalogue[cle];

let application: ChildProcess | undefined;
let navigateur: Browser | undefined;
let page: Page;
let dossier: string;

async function attendreWebView(): Promise<Browser> {
  for (let essai = 0; essai < 60; essai += 1) {
    try {
      return await chromium.connectOverCDP(`http://127.0.0.1:${PORT_DEBOGAGE}`);
    } catch {
      await new Promise((reprise) => setTimeout(reprise, 500));
    }
  }
  throw new Error(`WebView2 ne répond pas sur le port ${PORT_DEBOGAGE}`);
}

// L'espace de travail défile à l'intérieur d'une fenêtre de hauteur fixe : pour la capture
// seulement, on le déplie afin de photographier l'écran entier.
const DEPLIER = "html, body, #racine, .h-screen { height: auto !important; } main { overflow: visible !important; }";

async function capturer(nom: string) {
  const depli = await page.addStyleTag({ content: DEPLIER });
  await page.screenshot({ path: join(CAPTURES, `${nom}.png`), fullPage: true });
  await depli.evaluate((style) => style.parentNode?.removeChild(style));
}

/** Déplie les groupes repliés du menu (mise en place de l'année, outils). */
async function deplierMenu() {
  const navigation = page.getByRole("navigation");
  // Le menu peut ne pas être encore affiché (juste après un déverrouillage) : « Tableau de bord »,
  // toujours présent, dit qu'il l'est.
  await navigation.getByRole("button", { name: texte("nav.accueil"), exact: true }).waitFor();
  for (const groupe of ["nav.groupe.parametres", "nav.groupe.outils"] as const) {
    const entete = navigation.getByRole("button", { name: texte(groupe), exact: true });
    if ((await entete.count()) > 0 && (await entete.getAttribute("aria-expanded")) === "false") await entete.click();
  }
}

/** Va à une section du menu, en dépliant son groupe s'il est replié. */
async function allerA(cle: Cle) {
  await page.getByRole("navigation").getByRole("button", { name: texte("nav.accueil"), exact: true }).waitFor();
  const bouton = page.getByRole("navigation").getByRole("button", { name: texte(cle), exact: true });
  if (!(await bouton.isVisible())) await deplierMenu();
  await bouton.click();
}

test.describe.configure({ mode: "serial" });

test.beforeAll(async () => {
  // `GESCO_ECOLE_ESSAI` garde l'école construite par les scénarios dans ce dossier, pour l'ouvrir
  // ensuite à la main ; sans elle, un dossier jetable.
  dossier = process.env["GESCO_ECOLE_ESSAI"] ?? mkdtempSync(join(tmpdir(), "gesco-essai-"));
  application = spawn(EXECUTABLE, [], {
    env: {
      ...process.env,
      GESCO_DOSSIER_ESSAI: join(dossier, "donnees"),
      // Les scénarios vivent le 14 octobre 2026, quel que soit le jour réel : échéances, impayés et
      // relevés y sont datés.
      GESCO_DATE_ESSAI: JOUR_ESSAI,
      WEBVIEW2_USER_DATA_FOLDER: join(dossier, "webview"),
      // Une caméra simulée, acceptée sans question : la photo à la webcam s'essaie sans caméra.
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${PORT_DEBOGAGE} --use-fake-device-for-media-stream --use-fake-ui-for-media-stream`,
    },
    stdio: "ignore",
  });
  navigateur = await attendreWebView();
  const [contexte] = navigateur.contexts();
  if (contexte === undefined) throw new Error("WebView2 n'expose aucun contexte");
  page = contexte.pages()[0] ?? (await contexte.waitForEvent("page"));
});

test.afterAll(async () => {
  await navigateur?.close();
  // L'arbre entier : les processus WebView2 tiennent des fichiers du dossier d'essai.
  if (application?.pid !== undefined) spawnSync("taskkill", ["/pid", String(application.pid), "/T", "/F"]);
  // Les processus WebView2 relâchent parfois leurs fichiers après la fin de l'essai : le dossier
  // reste alors dans le répertoire temporaire, sans effet sur le résultat.
  if (process.env["GESCO_ECOLE_ESSAI"] !== undefined) return;
  try {
    rmSync(dossier, { recursive: true, force: true, maxRetries: 10, retryDelay: 500 });
  } catch {
    return;
  }
});

test("premier démarrage et phrase de secours", async () => {
  await expect(page.getByRole("heading", { name: texte("demarrage.titre") })).toBeVisible({ timeout: 30_000 });
  await capturer("01-premier-demarrage");

  const champs: [Cle, string][] = [
    ["demarrage.organisation", "Groupe scolaire Askia"],
    ["demarrage.etablissement_nom", "École fondamentale Askia"],
    ["demarrage.code_academie", "AE-BKO-RD"],
    ["demarrage.code_cap", "CAP-01"],
    ["demarrage.adresse", "Hamdallaye ACI 2000, Bamako"],
    ["demarrage.telephone", "+223 20 22 00 00"],
    ["demarrage.administrateur_identifiant", IDENTIFIANT],
    ["demarrage.administrateur_nom", NOM],
    ["demarrage.administrateur_mot_de_passe", MOT_DE_PASSE],
  ];
  for (const [cle, valeur] of champs) await page.getByLabel(texte(cle)).fill(valeur);
  // L'école d'essai enseigne le fondamental et le lycée, pas le jardin.
  for (const cycle of ["premier_cycle", "second_cycle", "lycee"]) await page.locator(`#cycle-${cycle}`).check();
  await capturer("02-premier-demarrage-rempli");
  await page.locator("form").getByRole("button", { name: texte("demarrage.creer") }).click();

  await expect(page.getByRole("heading", { name: texte("phrase.titre") })).toBeVisible({ timeout: 30_000 });
  // Le visionneur PDF de WebView2 rend la page hors du DOM : aucun signal à attendre. Sur un
  // écran mis à l'échelle (125 %), sa capture par CDP est agrandie et coupée ; à l'écran,
  // l'aperçu est entier (vérifié à l'œil le 7 octobre 2026).
  await page.waitForTimeout(4000);
  await capturer("03-phrase-de-secours");
  await page.getByLabel(texte("phrase.confirmation")).check();
  await page.getByRole("button", { name: texte("commun.continuer") }).click();
});

test("connexion", async () => {
  await expect(page.getByRole("heading", { name: texte("connexion.titre") })).toBeVisible();
  await capturer("04-connexion");
  // L'administrateur qui a perdu son mot de passe le remplace avec les douze mots.
  await page.getByRole("button", { name: texte("secours.lien") }).click();
  await expect(page.getByRole("heading", { name: texte("secours.titre") })).toBeVisible();
  await capturer("132-mot-de-passe-oublie");
  await page.getByRole("button", { name: texte("commun.annuler") }).click();
  await page.getByLabel(texte("connexion.identifiant")).fill(IDENTIFIANT);
  await page.getByLabel(texte("connexion.mot_de_passe")).fill(MOT_DE_PASSE);
  await page.getByRole("button", { name: texte("connexion.valider") }).click();
  await expect(page.getByText(texte("bandeau.utilisateur").replace("{nom}", NOM))).toBeVisible();
  // La case cochée aux douze mots suffit : le bandeau ne réclame plus le certificat.
  await expect(page.getByText(texte("avertissement.certificat_non_confirme"))).toHaveCount(0);
});

test("changement de mot de passe exigé, puis code PIN", async () => {
  await expect(page.getByText(texte("compte.changement_exige"))).toBeVisible();
  await capturer("05-changement-mot-de-passe-exige");
  const motDePasse = page.locator("form").first();
  await motDePasse.getByLabel(texte("compte.mot_de_passe_actuel")).fill(MOT_DE_PASSE);
  await motDePasse.getByLabel(texte("compte.nouveau_mot_de_passe")).fill(NOUVEAU_MOT_DE_PASSE);
  await motDePasse.getByRole("button", { name: texte("commun.enregistrer") }).click();
  await expect(page.getByText(texte("compte.changement_exige"))).toBeHidden();

  await allerA("nav.compte");
  const codePin = page.locator("form").nth(1);
  await codePin.getByLabel(texte("compte.mot_de_passe_actuel")).fill(NOUVEAU_MOT_DE_PASSE);
  await codePin.getByLabel(texte("compte.nouveau_pin")).fill(PIN);
  await codePin.getByRole("button", { name: texte("commun.enregistrer") }).click();
  await expect(codePin.getByRole("status")).toBeVisible();
});

test("sections de la barre latérale", async () => {
  const sections: [string, Cle][] = [
    ["06-tableau-de-bord", "nav.accueil"],
    ["07-mon-compte", "nav.compte"],
    ["08-utilisateurs", "nav.utilisateurs"],
    ["09-etablissement", "nav.parametres"],
    ["10-sauvegarde", "nav.sauvegarde"],
    ["11-journal", "nav.journal"],
    ["12-licence", "nav.licence"],
  ];
  for (const [nom, cle] of sections) {
    await page.getByRole("navigation").getByRole("button", { name: texte(cle) }).click();
    await page.waitForLoadState("networkidle");
    await capturer(nom);
  }
});

test("rôles personnalisés", async () => {
  await allerA("nav.roles");
  await expect(page.getByRole("heading", { name: texte("roles.titre") })).toBeVisible();
  await capturer("13-roles");

  await page.locator("#nouveau-role-nom").fill("Agent d'accueil");
  await page.locator("#nouveau-role-modele").selectOption("secretaire");
  await page.getByRole("button", { name: texte("roles.creer") }).click();
  const composition = texte("roles.composition").replace("{nom}", "Agent d'accueil");
  await expect(page.getByRole("heading", { name: composition })).toBeVisible();
  await capturer("14-role-personnalise");

  // Q-43 : cocher la saisie des notes et l'encaissement dans un même rôle est refusé.
  await page.locator('[id="permission-grades:create"]').check();
  await page.locator('[id="permission-payments:create"]').check();
  await page.getByRole("button", { name: texte("commun.enregistrer") }).click();
  await expect(page.getByText(texte("role.cumul_notes_caisse"))).toBeVisible();
  await capturer("15-cumul-notes-caisse-refuse");
});

test("comptes : création, réinitialisation", async () => {
  await allerA("nav.utilisateurs");
  await page.locator("#nouvel-identifiant").fill("fanta");
  await page.locator("#nouveau-nom").fill("Fanta Coulibaly");
  await page.locator("#nouveau-mdp-initial").fill("secretariat-2026");
  await page.locator("#nouveau-role-secretaire").check();
  await page.getByRole("button", { name: texte("utilisateurs.creer") }).click();
  await page.getByRole("cell", { name: "Fanta Coulibaly" }).click();
  await expect(page.getByRole("heading", { name: texte("utilisateurs.fiche").replace("{nom}", "Fanta Coulibaly") })).toBeVisible();
  await capturer("16-fiche-utilisateur");

  await page.locator("#fiche-reinitialisation").fill("nouveau-depart-2026");
  await page.getByRole("button", { name: texte("utilisateurs.reinitialiser") }).click();
  await expect(page.getByText(texte("utilisateurs.changement_attendu"))).toBeVisible();
  await capturer("17-mot-de-passe-reinitialise");
});

test("bascule d'utilisateur chronométrée", async () => {
  await page.getByRole("button", { name: texte("bandeau.verrouiller") }).click();
  await expect(page.getByRole("heading", { name: texte("verrou.titre") })).toBeVisible();
  await page.getByRole("button", { name: texte("verrou.autre_utilisateur") }).click();
  await page.getByLabel(texte("connexion.identifiant")).fill("fanta");
  await page.getByLabel(texte("connexion.mot_de_passe")).fill("nouveau-depart-2026");
  await page.getByRole("button", { name: texte("connexion.valider") }).click();
  const changement = page.locator("form").first();
  await changement.getByLabel(texte("compte.mot_de_passe_actuel")).fill("nouveau-depart-2026");
  await changement.getByLabel(texte("compte.nouveau_mot_de_passe")).fill("secretariat-rentree-2026");
  await changement.getByRole("button", { name: texte("commun.enregistrer") }).click();
  await expect(page.getByText(texte("compte.pin_absent"))).toBeVisible();
  await capturer("18-invitation-au-pin");

  await page.getByRole("button", { name: texte("bandeau.verrouiller") }).click();
  await page.getByRole("button", { name: /Fanta Coulibaly/ }).click();
  await expect(page.getByLabel(texte("verrou.mot_de_passe"))).toBeVisible();
  await capturer("19-ecran-verrouille-sans-pin");

  // RA-13 : de l'écran verrouillé à l'écran de travail en moins de trois secondes.
  await page.getByRole("button", { name: /Awa Traoré/ }).click();
  await page.getByLabel(texte("verrou.pin")).fill(PIN);
  const debut = Date.now();
  await page.getByRole("button", { name: texte("verrou.deverrouiller") }).click();
  await expect(page.getByText(texte("bandeau.utilisateur").replace("{nom}", NOM))).toBeVisible();
  const duree = Date.now() - debut;
  test.info().annotations.push({ type: "bascule_ms", description: String(duree) });
  expect(duree).toBeLessThan(3000);
});

test("fiche d'établissement et logo dans les documents", async () => {
  await allerA("nav.parametres");
  await page.locator("#etablissement-adresse").fill("Hamdallaye ACI 2000, Bamako");
  await page.locator("#etablissement-mentions").fill("Autorisation d'ouverture n° 2026-0042");
  await page.getByRole("button", { name: texte("commun.enregistrer") }).first().click();
  await expect(page.getByText(texte("commun.enregistre_a").split(" {")[0] ?? "")).toBeVisible();
  await page.locator("#etablissement-logo").setInputFiles(join(import.meta.dirname, "logo-essai.png"));
  await expect(page.getByRole("img", { name: texte("etablissement.logo") })).toBeVisible();
  await capturer("20-fiche-etablissement");

  // Le document d'essai, rarement utile, est en bas de l'écran « Établissement ».
  await page.getByRole("button", { name: texte("accueil.document_essai") }).click();
  await expect(page.locator("iframe")).toBeVisible({ timeout: 30_000 });
  await page.waitForTimeout(4000);
  await capturer("21-document-avec-logo");
});

test("année scolaire : préparer puis ouvrir", async () => {
  await allerA("nav.annees");
  await expect(page.getByText(texte("avertissement.annee_absente"))).toBeVisible();
  await page.locator("#nouvelle-annee-libelle").fill("2026-2027");
  await page.locator("#nouvelle-annee-debut").fill("2026-10-01");
  await page.locator("#nouvelle-annee-fin").fill("2027-07-31");
  await page.getByRole("button", { name: texte("annees.creer") }).click();
  await page.getByRole("cell", { name: "2026-2027" }).click();
  await expect(page.getByRole("button", { name: texte("annees.ouvrir") })).toBeVisible();
  await capturer("22-annee-en-preparation");

  await page.getByRole("button", { name: texte("annees.ouvrir") }).click();
  await expect(page.getByText(texte("bandeau.annee").replace("{annee}", "2026-2027"), { exact: false })).toBeVisible();
  await expect(page.getByText(texte("avertissement.annee_absente"))).toBeHidden();
  await capturer("23-annee-ouverte");
});

test("journal d'audit filtré, détail avant et après", async () => {
  await allerA("nav.journal");
  await page.locator("#journal-entite").selectOption("role");
  await expect(page.getByRole("cell", { name: texte("entite.role") }).first()).toBeVisible();
  await page.getByRole("cell", { name: texte("entite.role") }).first().click();
  await expect(page.getByRole("columnheader", { name: texte("journal.apres") })).toBeVisible();
  await capturer("24-journal-filtre-et-detail");
  await page.getByRole("button", { name: texte("journal.verifier") }).click();
  await expect(page.getByRole("status").filter({ hasText: texte("journal.integre").split(" :")[0] ?? "" })).toBeVisible();
});

test("une secrétaire ne voit que ses sections ; un compte désactivé perd sa session", async () => {
  await page.getByRole("button", { name: texte("bandeau.verrouiller") }).click();
  await page.getByRole("button", { name: /Fanta Coulibaly/ }).click();
  await page.getByLabel(texte("verrou.mot_de_passe")).fill("secretariat-rentree-2026");
  await page.getByRole("button", { name: texte("verrou.deverrouiller") }).click();
  await expect(page.getByText(texte("bandeau.utilisateur").replace("{nom}", "Fanta Coulibaly"))).toBeVisible();
  const navigation = page.getByRole("navigation");
  await deplierMenu();
  for (const cle of ["nav.utilisateurs", "nav.roles", "nav.parametres", "nav.journal", "nav.licence"] as const) {
    await expect(navigation.getByRole("button", { name: texte(cle) })).toHaveCount(0);
  }
  await capturer("25-espace-de-la-secretaire");

  // L'administratrice désactive le compte de Fanta pendant que sa session est ouverte.
  await page.getByRole("button", { name: texte("bandeau.verrouiller") }).click();
  await page.getByRole("button", { name: /Awa Traoré/ }).click();
  await page.getByLabel(texte("verrou.pin")).fill(PIN);
  await page.getByRole("button", { name: texte("verrou.deverrouiller") }).click();
  await allerA("nav.utilisateurs");
  await page.getByRole("cell", { name: "Fanta Coulibaly" }).click();
  await page.locator("#fiche-actif").uncheck();
  await page.locator("form").filter({ has: page.locator("#fiche-actif") }).getByRole("button", { name: texte("commun.enregistrer") }).click();
  await expect(page.getByRole("cell", { name: texte("commun.non") })).toBeVisible();
  await page.getByRole("button", { name: texte("bandeau.verrouiller") }).click();
  await expect(page.getByRole("button", { name: /Awa Traoré/ })).toBeVisible();
  await expect(page.getByRole("button", { name: /Fanta Coulibaly/ })).toHaveCount(0);
  await capturer("26-session-du-compte-desactive-fermee");
});

test("phase 2 : niveaux renommés et classes de l'année", async () => {
  await page.getByRole("button", { name: /Awa Traoré/ }).click();
  await page.getByLabel(texte("verrou.pin")).fill(PIN);
  await page.getByRole("button", { name: texte("verrou.deverrouiller") }).click();
  await allerA("nav.niveaux");
  await page.getByRole("cell", { name: "7e année" }).click();
  await page.locator("#referentiel-libelle").fill("7ème année");
  await page.locator("form").filter({ has: page.locator("#referentiel-libelle") }).getByRole("button", { name: texte("commun.enregistrer") }).click();
  await expect(page.getByRole("cell", { name: "7ème année" })).toBeVisible();
  await capturer("27-niveaux-et-series");

  await allerA("nav.classes");
  const classes: [string, string | null, string][] = [
    ["7ème année", null, "7e A"],
    ["11e année", "S — Sciences", "11S"],
    ["12e année", "TSE — Sciences exactes", "12TSE"],
  ];
  for (const [niveau, serie, libelle] of classes) {
    await page.locator("#classe-niveau").selectOption({ label: niveau });
    if (serie !== null) await page.locator("#classe-serie").selectOption({ label: serie });
    await page.locator("#classe-libelle").fill(libelle);
    await page.getByRole("button", { name: texte("classes.creer") }).click();
    await expect(page.getByRole("cell", { name: libelle, exact: true })).toBeVisible();
  }
  // Une classe de 11e sans série est refusée (SPEC_PHASE_2 §3.1).
  await page.locator("#classe-niveau").selectOption({ label: "11e année" });
  await page.locator("#classe-libelle").fill("11 sans série");
  await page.getByRole("button", { name: texte("classes.creer") }).click();
  await expect(page.getByText(texte("classe.serie_exigee"))).toBeVisible();
  await capturer("28-classes-de-l-annee");
});

test("phase 3 : la direction saisit les frais de l'année et les met en vigueur", async () => {
  await allerA("nav.frais");
  await page.getByRole("button", { name: texte("frais.nouvelle") }).click();
  // Grille neuve : les trois plans livrés, « Trois tranches » par défaut.
  await expect(page.locator("#frais-plan-1-libelle")).toHaveValue("Trois tranches");
  await expect(page.locator("#frais-plan-1-defaut")).toBeChecked();
  // Les échéances livrées tombent le 5 ; un clic les met toutes à un autre jour de leur mois.
  await expect(page.locator("#frais-plan-1-tranche-0-echeance")).toHaveValue("2026-10-05");
  await page.locator("#frais-jour-echeance").fill("10");
  await page.getByRole("button", { name: texte("frais.aligner_jour") }).click();
  await expect(page.locator("#frais-plan-2-tranche-8-echeance")).toHaveValue("2027-06-10");
  await page.locator("#frais-jour-echeance").fill("5");
  await page.getByRole("button", { name: texte("frais.aligner_jour") }).click();
  await expect(page.locator("#frais-plan-1-tranche-0-echeance")).toHaveValue("2026-10-05");

  await page.getByRole("button", { name: texte("frais.ajouter_ligne") }).click();
  await page.locator("#frais-ligne-0-niveau-7").check();
  // La 3e année aussi : sa classe sert à l'UC-04 du 1er cycle.
  await page.locator("#frais-ligne-0-niveau-3").check();
  await page.locator("#frais-ligne-0-montant").fill("150000");
  await page.getByRole("button", { name: texte("frais.ajouter_ligne") }).click();
  await page.locator("#frais-ligne-1-niveau-7").check();
  await page.locator("#frais-ligne-1-niveau-3").check();
  await page.locator("#frais-ligne-1-type").selectOption("inscription");
  await page.locator("#frais-ligne-1-montant").fill("10000");
  await page.getByRole("button", { name: texte("commun.enregistrer") }).click();
  await expect(page.getByText(/Enregistré à/)).toBeVisible();
  await capturer("50-frais-de-l-annee");

  await page.getByRole("button", { name: texte("frais.mettre_en_vigueur") }).click();
  // Les niveaux qui ont des classes sans scolarité sont signalés, sans blocage.
  await expect(page.getByRole("status").filter({ hasText: "11e année" })).toBeVisible();
  await expect(page.getByRole("heading", { name: `Frais 2026-2027 · ${texte("frais.etat.en_vigueur")}` })).toBeVisible();
  await expect(page.getByRole("cell", { name: "150 000 F" })).toBeVisible();
  await capturer("51-frais-en-vigueur");
});

test("phase 2 : fiche élève, responsable retrouvé par son téléphone, photo", async () => {
  await allerA("nav.eleves");

  const creerEleve = async (nom: string, prenoms: string, sexe: string, naissance: string) => {
    await page.getByRole("button", { name: texte("eleves.nouveau") }).click();
    await page.locator("#nouvel-eleve-nom").fill(nom);
    await page.locator("#nouvel-eleve-prenoms").fill(prenoms);
    await page.locator("#nouvel-eleve-sexe").selectOption(sexe);
    await page.locator("#nouvel-eleve-naissance").fill(naissance);
    await page.locator("#nouvel-eleve-lieu_naissance").fill("Bamako");
    await page.getByRole("button", { name: texte("eleves.creer") }).click();
    await expect(page.getByRole("heading", { name: `${prenoms} ${nom}` })).toBeVisible();
  };

  await page.getByRole("button", { name: texte("eleves.nouveau") }).click();
  await page.locator("#nouvel-eleve-nom").fill("Traoré");
  await page.locator("#nouvel-eleve-prenoms").fill("Awa");
  await capturer("29-nouvel-eleve");
  await page.locator("#nouvel-eleve-sexe").selectOption("feminin");
  await page.locator("#nouvel-eleve-naissance").fill("2014-03-02");
  await page.locator("#nouvel-eleve-lieu_naissance").fill("Bamako");
  await page.getByRole("button", { name: texte("eleves.creer") }).click();
  await expect(page.getByRole("heading", { name: "Awa Traoré" })).toBeVisible();
  await expect(page.locator("aside").getByText("2026/00001")).toBeVisible();

  // La mère, saisie sous une forme quelconque, est enregistrée sous la forme canonique.
  await page.locator("#ajout-lien").selectOption("mere");
  await page.locator("#nouveau-responsable-nom").fill("Coulibaly");
  await page.locator("#nouveau-responsable-prenoms").fill("Mariam");
  await page.locator("#nouveau-responsable-telephone-0").fill("76 12 34 56");
  await page.getByRole("button", { name: texte("responsables.creer_et_lier") }).click();
  await expect(page.locator("aside").getByText("+223 76 12 34 56")).toBeVisible();
  await page.locator("#eleve-photo").setInputFiles(join(import.meta.dirname, "logo-essai.png"));
  await expect(page.getByRole("img", { name: texte("eleves.photo") })).toBeVisible();
  await capturer("30-fiche-eleve");

  // Le frère : la mère se retrouve par son téléphone et se lie sans ressaisie.
  await creerEleve("Traoré", "Moussa", "masculin", "2016-09-12");
  await page.locator("#ajout-lien").selectOption("mere");
  await page.locator("#responsable-recherche").fill("0022376123456");
  await expect(page.getByText(texte("responsables.deja_parent_de").replace("{enfants}", "Awa Traoré"))).toBeVisible();
  await capturer("31-responsable-retrouve");
  await page.getByRole("button", { name: texte("responsables.lier") }).click();
  await expect(page.locator("aside").getByText("+223 76 12 34 56")).toBeVisible();

  await page.locator("#eleves-recherche").fill("traore");
  await expect(page.getByRole("cell", { name: "2026/00002" })).toBeVisible();
  await capturer("32-liste-des-eleves");
});

test("phase 2 : matricule saisi par l'école", async () => {
  await allerA("nav.parametres");
  const caseManuelle = page.locator('[id="param-matricule.saisie_manuelle"]');
  await caseManuelle.check();
  const ligne = caseManuelle.locator("xpath=ancestor::div[contains(@class, 'border-b')]");
  await ligne.getByRole("button", { name: texte("commun.enregistrer") }).click();
  await expect(page.locator('[id="param-matricule.saisie_manuelle"]')).toBeChecked();
  await capturer("33-regles-du-matricule");

  await allerA("nav.eleves");
  await page.getByRole("button", { name: texte("eleves.nouveau") }).click();
  await page.locator("#nouvel-eleve-matricule").fill("L/2013/042");
  await page.locator("#nouvel-eleve-nom").fill("Diarra");
  await page.locator("#nouvel-eleve-prenoms").fill("Seydou");
  await page.locator("#nouvel-eleve-sexe").selectOption("masculin");
  await page.locator("#nouvel-eleve-naissance").fill("2013-06-21");
  await capturer("34-matricule-saisi");
  await page.getByRole("button", { name: texte("eleves.creer") }).click();
  await expect(page.locator("aside").getByText("L/2013/042").first()).toBeVisible();
});

test("phase 2 : doublon signalé, recherche globale au clavier", async () => {
  await allerA("nav.eleves");
  await page.getByRole("button", { name: texte("eleves.nouveau") }).click();
  await page.locator("#nouvel-eleve-matricule").fill("L/2014/001");
  await page.locator("#nouvel-eleve-nom").fill("TRAORE");
  await page.locator("#nouvel-eleve-prenoms").fill("Hawa");
  await page.locator("#nouvel-eleve-naissance").fill("2014-05-10");
  await expect(page.getByText(texte("eleves.doublons"))).toBeVisible();
  await expect(page.getByRole("button", { name: /Awa Traoré/ })).toBeVisible();
  await capturer("35-doublon-signale");

  // Ctrl+K, une orthographe approximative, Entrée : la fiche s'ouvre sans la souris.
  await page.keyboard.press("Control+k");
  const recherche = page.getByRole("dialog", { name: texte("recherche.titre") });
  await recherche.getByRole("searchbox").fill("Kouloubaly");
  await expect(recherche.getByText("Mariam Coulibaly")).toBeVisible();
  await capturer("36-recherche-globale");
  await page.keyboard.press("Enter");
  await expect(page.getByRole("heading", { name: "Awa Traoré" })).toBeVisible();
});

test("phase 2 : validation refusée sur une identité incomplète, puis assistant d'inscription UC-01", async () => {
  // Seydou Diarra a été créé sans lieu de naissance : le brouillon reste, la raison s'affiche.
  await allerA("nav.eleves");
  await page.locator("#eleves-recherche").fill("Diarra Seydou");
  await page.getByRole("cell", { name: "Seydou" }).click();
  await page.locator("#inscription-classe").selectOption({ label: "7e A · 7ème année" });
  await page.getByRole("button", { name: texte("inscriptions.inscrire_et_valider") }).click();
  await expect(page.getByText(texte("inscription.identite_incomplete"))).toBeVisible();
  await expect(page.getByRole("button", { name: texte("inscriptions.valider") })).toBeVisible();
  await capturer("37-validation-refusee");

  // UC-01 : un nouvel élève, son père, sa classe, validée, sans quitter l'écran.
  await allerA("nav.inscriptions");
  const debut = Date.now();
  await page.getByRole("button", { name: texte("inscriptions.assistant") }).click();
  await page.locator("#nouvel-eleve-matricule").fill("L/2015/007");
  await page.locator("#nouvel-eleve-nom").fill("Sangaré");
  await page.locator("#nouvel-eleve-prenoms").fill("Oumar");
  await page.locator("#nouvel-eleve-sexe").selectOption("masculin");
  await page.locator("#nouvel-eleve-naissance").fill("2015-01-20");
  await page.locator("#nouvel-eleve-lieu_naissance").fill("Ségou");
  await page.getByRole("button", { name: texte("eleves.creer") }).click();

  await page.locator("#ajout-lien").selectOption("pere");
  await page.locator("#nouveau-responsable-nom").fill("Sangaré");
  await page.locator("#nouveau-responsable-prenoms").fill("Ibrahim");
  await page.locator("#nouveau-responsable-telephone-0").fill("66 00 11 22");
  await page.getByRole("button", { name: texte("responsables.creer_et_lier") }).click();
  await expect(page.getByText("+223 66 00 11 22")).toBeVisible();
  await capturer("38-assistant-responsables");
  await page.getByRole("button", { name: texte("commun.continuer") }).click();

  await page.locator("#inscription-classe").selectOption({ label: "7e A · 7ème année" });
  await page.getByRole("button", { name: texte("inscriptions.inscrire_et_valider") }).click();
  await expect(page.getByText(`2026-2027 · 7e A · ${texte("inscriptions.statut.validee")}`)).toBeVisible();
  const duree = Date.now() - debut;
  test.info().annotations.push({ type: "uc01_ms", description: String(duree) });
  writeFileSync(join(CAPTURES, "uc01-chronometre.txt"), `UC-01 : ${duree} ms, de l'ouverture de l'assistant à la validation, piloté par Playwright (borne basse ; cible : moins de 5 minutes pour une secrétaire au clavier).\n`, "utf8");
  expect(duree).toBeLessThan(5 * 60_000);
  await capturer("39-inscription-validee");

  await page.getByRole("button", { name: texte("inscriptions.terminer") }).click();
  await expect(page.getByRole("cell", { name: "Oumar Sangaré" })).toBeVisible();
  await capturer("40-inscriptions-de-l-annee");
});

test("phase 2 : fiche d'inscription imprimée, photo et QR code", async () => {
  // La liste des inscriptions ouvre la fiche de l'élève.
  await page.getByRole("cell", { name: "Oumar Sangaré" }).click();
  await expect(page.getByRole("heading", { name: "Oumar Sangaré" })).toBeVisible();
  // La validation de l'assistant a créé la facture et l'échéancier (SPEC_PHASE_3 §4.1).
  await expect(page.getByText(/FAC-2026-000001/)).toBeVisible();
  await expect(page.getByText("Scolarité · 2e tranche")).toBeVisible();
  await page.getByText(/FAC-2026-000001/).scrollIntoViewIfNeeded();
  await capturer("52-frais-sur-la-fiche");
  await page.locator("#eleve-photo").setInputFiles(join(import.meta.dirname, "logo-essai.png"));
  await expect(page.getByRole("img", { name: texte("eleves.photo") })).toBeVisible();
  await page.getByRole("button", { name: texte("inscriptions.imprimer_fiche") }).click();
  await expect(page.locator("iframe")).toBeVisible({ timeout: 30_000 });
  // Le visionneur PDF de WebView2 rend la page hors du DOM : aucun signal à attendre.
  await page.waitForTimeout(4000);
  await page.locator("iframe").scrollIntoViewIfNeeded();
  await capturer("41-fiche-d-inscription");
});

test("phase 3 : remise accordée par la direction sur les dernières tranches", async () => {
  await allerA("nav.eleves");
  await page.locator("#eleves-recherche").fill("Oumar");
  await page.getByRole("cell", { name: "Oumar" }).click();
  await expect(page.getByRole("heading", { name: "Oumar Sangaré" })).toBeVisible();
  // La direction propose et accorde d'un seul geste : 25 % de la scolarité, motif obligatoire.
  await page.getByRole("button", { name: texte("remise.nouvelle_accordee") }).click();
  const formulaire = page.locator("div").filter({ has: page.locator("[id^=remise-motif-]") }).last();
  await formulaire.locator("[id^=remise-valeur-]").fill("25");
  await formulaire.locator("[id^=remise-motif-]").fill("Deux frères déjà inscrits");
  await page.getByRole("button", { name: texte("remise.proposer_accorder") }).click();
  await expect(page.getByText("Fratrie de 25 % sur Scolarité : accordée")).toBeVisible();
  // 37 500 F pris sur la dernière tranche de 45 000 F : il en reste 7 500.
  await expect(page.getByRole("cell", { name: "− 37 500 F" })).toBeVisible();
  await expect(page.getByRole("cell", { name: "7 500 F", exact: true })).toBeVisible();
  await page.getByText("Fratrie de 25 % sur Scolarité : accordée").scrollIntoViewIfNeeded();
  await capturer("53-remise-accordee");
});

test("phase 3 : la caisse s'ouvre avec son fonds, se ferme avec son écart expliqué, puis se vise", async () => {
  await allerA("nav.caisse");
  await page.locator("#caisse-fonds").fill("25000");
  await page.getByRole("button", { name: texte("caisse.ouvrir_bouton") }).click();
  await expect(page.getByRole("heading", { name: /Caisse ouverte depuis le/ })).toBeVisible();
  // Il manque 500 F dans le tiroir : l'écart s'affiche et doit être expliqué.
  await page.locator("#caisse-compte").fill("24500");
  await expect(page.getByRole("status").filter({ hasText: "Écart" })).toBeVisible();
  await page.getByRole("button", { name: texte("caisse.cloturer") }).click();
  await expect(page.getByText(texte("caisse.motif_ecart_obligatoire"))).toBeVisible();
  await page.locator("#caisse-motif").fill("Monnaie rendue en trop à un parent");
  await capturer("54-fermeture-de-caisse");
  await page.getByRole("button", { name: texte("caisse.cloturer") }).click();
  await expect(page.getByRole("heading", { name: texte("caisse.cloturee_ok") })).toBeVisible();

  // La direction vise la session et imprime l'état de caisse.
  await allerA("nav.sessions_caisse");
  await page.getByRole("cell", { name: texte("caisse.statut.cloturee") }).click();
  await page.locator("#caisse-visa").fill("Écart vérifié avec la caissière");
  await page.getByRole("button", { name: texte("caisse.viser") }).click();
  await expect(page.getByRole("cell", { name: texte("caisse.statut.rapprochee") })).toBeVisible();
  await capturer("55-sessions-de-caisse");
});

test("phase 3 : UC-02 chronométré, un versement encaissé et son reçu en moins d'une minute", async () => {
  await allerA("nav.caisse");
  await page.locator("#caisse-fonds").fill("0");
  await page.getByRole("button", { name: texte("caisse.ouvrir_bouton") }).click();
  await expect(page.locator("#encaissement-recherche")).toBeVisible();

  const debut = Date.now();
  await page.locator("#encaissement-recherche").fill("Oumar");
  await page.getByRole("button", { name: /Oumar Sangaré/ }).click();
  await expect(page.getByText(/Reste à payer/)).toBeVisible();
  await page.locator("#encaissement-montant").fill("50000");
  // La plus ancienne d'abord : la 1re tranche, échue le 5 octobre, reçoit les 50 000 F.
  await expect(page.getByRole("row").filter({ hasText: "1re tranche" }).getByRole("cell", { name: "50 000 F", exact: true })).toBeVisible();
  await capturer("56-encaissement");
  await page.locator("#encaissement-montant").press("Enter");
  await expect(page.getByRole("status").filter({ hasText: "REC-2026-000001" })).toBeVisible();
  const duree = Date.now() - debut;
  test.info().annotations.push({ type: "uc02_ms", description: String(duree) });
  writeFileSync(join(CAPTURES, "uc02-chronometre.txt"), `UC-02 : ${duree} ms, de la recherche de l'élève au reçu affiché, piloté par Playwright (borne basse ; cible : moins de 60 secondes au clavier).\n`, "utf8");
  expect(duree).toBeLessThan(60_000);
  await expect(page.locator("iframe")).toBeVisible({ timeout: 30_000 });
  await page.waitForTimeout(4000);
  await capturer("57-recu");

  // Un paiement Orange Money : la référence est exigée.
  await page.getByRole("button", { name: texte("encaissement.nouveau") }).click();
  await page.locator("#encaissement-recherche").fill("Oumar");
  await page.getByRole("button", { name: /Oumar Sangaré/ }).click();
  await page.locator("#encaissement-montant").fill("10000");
  await page.locator("#encaissement-mode").selectOption("orange_money");
  await page.getByRole("button", { name: /Encaisser 10 000 F/ }).click();
  await expect(page.getByText(texte("versement.reference_obligatoire"))).toBeVisible();
  await page.locator("#encaissement-reference").fill("OM-2026-55871");
  await page.getByRole("button", { name: /Encaisser 10 000 F/ }).click();
  await expect(page.getByRole("status").filter({ hasText: "REC-2026-000002" })).toBeVisible();
});

test("phase 3 : un versement annulé avec son motif, son reçu marqué annulé", async () => {
  await allerA("nav.eleves");
  await page.locator("#eleves-recherche").fill("Oumar");
  await page.getByRole("cell", { name: "Oumar" }).click();
  await expect(page.getByRole("heading", { name: "Oumar Sangaré" })).toBeVisible();
  const ligne = page.getByRole("listitem").filter({ hasText: "Reçu REC-2026-000002" });
  await ligne.getByRole("button", { name: texte("versement.annuler") }).click();
  // Sans motif, rien ne part.
  await expect(ligne.getByRole("button", { name: texte("versement.confirmer_annulation") })).toBeDisabled();
  await ligne.locator("[id^=versement-motif-]").fill("Payé en espèces, pas par Orange Money");
  await ligne.getByRole("button", { name: texte("versement.confirmer_annulation") }).click();
  await expect(ligne.getByText("Annulé : Payé en espèces, pas par Orange Money")).toBeVisible();
  // La caisse est encore ouverte : simple contre-passation, pas de sortie de régularisation.
  await expect(ligne.getByText(/Sortie de régularisation/)).toHaveCount(0);
  await expect(ligne.locator("iframe")).toBeVisible({ timeout: 30_000 });
  await page.waitForTimeout(4000);
  await ligne.getByText("Annulé : Payé en espèces").scrollIntoViewIfNeeded();
  await capturer("58-versement-annule");

  // Le reçu annulé ne compte plus dans la caisse : un seul reçu, plus d'Orange Money.
  await allerA("nav.caisse");
  await expect(page.getByRole("definition").filter({ hasText: /^1$/ })).toBeVisible();
  await expect(page.getByRole("term").filter({ hasText: "Orange Money" })).toHaveCount(0);
});

test("phase 2 : fusion de deux fiches en doublon", async () => {
  await allerA("nav.eleves");
  // Le doublon a été signalé ; la secrétaire crée quand même la fiche, puis la fusionne.
  await page.getByRole("button", { name: texte("eleves.nouveau") }).click();
  await page.locator("#nouvel-eleve-matricule").fill("L/2014/001");
  await page.locator("#nouvel-eleve-nom").fill("TRAORE");
  await page.locator("#nouvel-eleve-prenoms").fill("Hawa");
  await page.locator("#nouvel-eleve-sexe").selectOption("feminin");
  await page.locator("#nouvel-eleve-naissance").fill("2014-05-10");
  await page.locator("#nouvel-eleve-lieu_naissance").fill("Kati");
  await page.getByRole("button", { name: texte("eleves.creer") }).click();
  await expect(page.getByRole("heading", { name: "Hawa TRAORE" })).toBeVisible();

  await page.getByRole("button", { name: texte("fusion.ouvrir") }).click();
  await page.getByRole("button", { name: /Awa Traoré · 2026\/00001/ }).click();
  // On garde la fiche la plus ancienne, celle d'Awa : la nouvelle sera absorbée.
  await page.locator("#fusion-garder").uncheck();
  await expect(page.getByRole("columnheader", { name: texte("fusion.resultat") })).toBeVisible();
  await expect(page.getByRole("cell", { name: "Kati" }).last()).toBeVisible();
  await capturer("42-apercu-de-la-fusion");
  await page.locator("#fusion-verifie").check();
  await page.getByRole("button", { name: texte("fusion.confirmer") }).click();
  await expect(page.getByRole("heading", { name: "Awa Traoré" })).toBeVisible();

  // L'ancien matricule mène désormais à la fiche gardée.
  await page.locator("#eleves-recherche").fill("L/2014/001");
  await expect(page.getByRole("cell", { name: "2026/00001" })).toBeVisible();
  await expect(page.getByRole("cell", { name: "L/2014/001" })).toHaveCount(0);
  await capturer("43-fiche-apres-fusion");
});

test("phase 2 : export Excel d'une liste", async () => {
  await allerA("nav.inscriptions");
  await expect(page.getByRole("cell", { name: "Oumar Sangaré" })).toBeVisible();
  // Le dialogue d'enregistrement du système ne se pilote pas. Tauri envoie ses appels par le
  // `fetch` de la page vers `ipc.localhost`, hors de vue de Playwright : on ne substitue que
  // l'appel du dialogue, qui reçoit un chemin du dossier d'essai ; tout le reste, dont la
  // commande `exporter_xlsx`, passe par le vrai noyau.
  const classeur = join(dossier, "inscriptions.xlsx");
  await page.evaluate((chemin) => {
    const fenetre = window as unknown as { fetchOriginal?: typeof fetch };
    const original = window.fetch.bind(window);
    fenetre.fetchOriginal = original;
    window.fetch = (entree, options) => {
      const url = typeof entree === "string" ? entree : entree instanceof URL ? entree.href : entree.url;
      if (decodeURIComponent(url).includes("ipc.localhost/plugin:dialog|save")) {
        return Promise.resolve(new Response(JSON.stringify(chemin), { status: 200, headers: { "Tauri-Response": "ok", "content-type": "application/json" } }));
      }
      return original(entree, options);
    };
  }, classeur);
  await page.getByRole("button", { name: texte("commun.exporter_excel") }).click();
  await expect(page.getByRole("status")).toContainText("inscriptions.xlsx");
  await page.evaluate(() => {
    const fenetre = window as unknown as { fetchOriginal?: typeof fetch };
    if (fenetre.fetchOriginal !== undefined) window.fetch = fenetre.fetchOriginal;
  });
  await capturer("44-export-excel");
  // Un classeur Office est une archive ZIP : « PK » en tête.
  const octets = readFileSync(classeur);
  expect(octets.subarray(0, 2).toString("latin1")).toBe("PK");
  expect(octets.length).toBeGreaterThan(1000);
});

test("phase 2 : import d'un fichier de quinze élèves, deux erreurs et un doublon", async () => {
  await allerA("nav.eleves");
  const fichier = join(dossier, "eleves.csv");
  const lignes = [
    "N°;Nom et prénoms;Sexe;Né(e) le;Lieu de naissance;Classe;Téléphone du parent;Matricule",
    "1;KEITA Fatoumata;F;01/01/2014;Ségou;7e A;70 00 00 01;L/2026/001",
    "2;DIALLO Moussa;M;2013-11-02;Bamako;7e A;66 00 11 22;L/2026/002",
    "3;COULIBALY Seydou;M;vers 2013;;7e A;70 00 00 01;L/2026/003",
    "4;SANGARE Aïcha;F;20/01/2013;Ségou;7e A;70 00 00 03;L/2026/004",
    "5;DEMBELE Mariam;F;03/03/2013;Koutiala;7e A;70 00 00 04;L/2026/005",
    "6;KONE Adama;M;;Sikasso;7e A;70 00 00 05;L/2026/006",
    "7;TOURE Aminata;F;14/07/2013;Mopti;9e A;70 00 00 06;L/2026/007",
    "8;SIDIBE Ibrahim;M;05/05/2014;Bamako;7e A;70 00 00 07;L/2026/008",
    "9;CISSE Kadiatou;F;06/06/2014;Bamako;7e A;70 00 00 08;L/2026/009",
    "10;KEITA Fatoumata;F;01/01/2014;Ségou;7e A;70 00 00 01;L/2026/010",
    "11;MAIGA Hamidou;M;07/07/2013;Gao;7e A;70 00 00 09;L/2026/011",
    "12;DIARRA Rokia;F;08/08/2013;Bamako;7e A;70 00 00 10;L/2026/012",
    "13;SAMAKE Bakary;M;09/09/2014;Bamako;7e A;70 00 00 11;L/2026/013",
    "14;BAGAYOKO Oumou;F;10/10/2014;Bamako;7e A;70 00 00 12;L/2026/014",
    "15;SOW Boubacar;M;11/11/2014;Bamako;7e A;70 00 00 13;L/2026/015",
  ];
  writeFileSync(fichier, `\uFEFF${lignes.join("\r\n")}\r\n`, "utf8");
  // Le dialogue d'ouverture du système ne se pilote pas : son appel reçoit le chemin du fichier.
  await page.evaluate((chemin) => {
    const fenetre = window as unknown as { fetchOriginal?: typeof fetch };
    const original = window.fetch.bind(window);
    fenetre.fetchOriginal = original;
    window.fetch = (entree, options) => {
      const url = typeof entree === "string" ? entree : entree instanceof URL ? entree.href : entree.url;
      if (decodeURIComponent(url).includes("ipc.localhost/plugin:dialog|open")) {
        return Promise.resolve(new Response(JSON.stringify(chemin), { status: 200, headers: { "Tauri-Response": "ok", "content-type": "application/json" } }));
      }
      return original(entree, options);
    };
  }, fichier);
  await page.getByRole("button", { name: texte("import.ouvrir") }).click();
  await page.getByRole("button", { name: texte("import.choisir") }).click();
  await expect(page.getByText("1 feuille(s), 15 ligne(s) lues.")).toBeVisible();
  await page.evaluate(() => {
    const fenetre = window as unknown as { fetchOriginal?: typeof fetch };
    if (fenetre.fetchOriginal !== undefined) window.fetch = fenetre.fetchOriginal;
  });
  // Les colonnes sont reconnues d'elles-mêmes.
  await expect(page.locator("#import-champ-1")).toHaveValue("nom_et_prenoms");
  await expect(page.locator("#import-champ-3")).toHaveValue("date_naissance");
  await expect(page.locator("#import-champ-6")).toHaveValue("responsable_telephone");
  await expect(page.locator("#import-champ-7")).toHaveValue("matricule_interne");
  await capturer("45-import-colonnes");
  await page.getByRole("button", { name: texte("import.verifier") }).click();
  await expect(page.getByRole("status")).toContainText("11 ligne(s) prête(s), 2 à vérifier après l'import, 2 en erreur");
  await expect(page.getByRole("alert")).toContainText("9e A");
  await capturer("46-import-apercu");
  await page.getByRole("button", { name: "Importer 13 élève(s)" }).click();
  await expect(page.getByRole("status")).toContainText("13 élève(s) créé(s)");
  await capturer("47-import-termine");
  await page.getByRole("button", { name: texte("import.fermer") }).click();
  await page.locator("#eleves-recherche").fill("Seydou");
  await expect(page.getByRole("cell", { name: "COULIBALY" })).toBeVisible();
});

test("phase 2 : au jardin, l'inscription est refusée sans photo puis acceptée", async () => {
  // L'école ouvre le jardin d'enfants : le niveau Petite section devient disponible.
  await allerA("nav.niveaux");
  await page.locator("#cycle-jardin").check();
  await page.locator("form").filter({ has: page.locator("#cycle-jardin") }).getByRole("button", { name: texte("commun.enregistrer") }).click();
  await expect(page.getByRole("cell", { name: "Petite section" })).toBeVisible();
  await allerA("nav.classes");
  await page.locator("#classe-niveau").selectOption({ label: "Petite section" });
  await page.locator("#classe-libelle").fill("PS");
  await page.getByRole("button", { name: texte("classes.creer") }).click();
  await expect(page.getByRole("cell", { name: "PS", exact: true })).toBeVisible();
  // La grille en vigueur ne couvre pas la petite section : une nouvelle version lui ajoute ses frais.
  await allerA("nav.frais");
  await page.getByRole("button", { name: texte("frais.nouvelle_version") }).click();
  await expect(page.locator("#frais-ligne-1-montant")).toHaveValue("10000");
  // Les frais d'inscription valent aussi pour la petite section.
  await page.locator("#frais-ligne-1-niveau-PS").check();
  await page.getByRole("button", { name: texte("frais.ajouter_ligne") }).click();
  await page.locator("#frais-ligne-2-niveau-PS").check();
  await page.locator("#frais-ligne-2-montant").fill("75000");
  await page.getByRole("button", { name: texte("frais.mettre_en_vigueur") }).click();
  await expect(page.getByRole("heading", { name: `Frais 2026-2027 (version 2) · ${texte("frais.etat.en_vigueur")}` })).toBeVisible();

  // Un petit de trois ans, sa mère qui vient le chercher, mais pas de photo : refus.
  await allerA("nav.eleves");
  await page.getByRole("button", { name: texte("eleves.nouveau") }).click();
  await page.locator("#nouvel-eleve-matricule").fill("J/2023/001");
  await page.locator("#nouvel-eleve-nom").fill("Keïta");
  await page.locator("#nouvel-eleve-prenoms").fill("Bakary");
  await page.locator("#nouvel-eleve-sexe").selectOption("masculin");
  await page.locator("#nouvel-eleve-naissance").fill("2023-02-11");
  await page.locator("#nouvel-eleve-lieu_naissance").fill("Bamako");
  await page.getByRole("button", { name: texte("eleves.creer") }).click();
  await expect(page.getByRole("heading", { name: "Bakary Keïta" })).toBeVisible();
  await page.locator("#ajout-lien").selectOption("mere");
  await page.locator("#ajout-vient_chercher").check();
  await page.locator("#nouveau-responsable-nom").fill("Keïta");
  await page.locator("#nouveau-responsable-prenoms").fill("Assitan");
  await page.locator("#nouveau-responsable-telephone-0").fill("79 00 00 01");
  await page.getByRole("button", { name: texte("responsables.creer_et_lier") }).click();
  await expect(page.locator("aside").getByText("+223 79 00 00 01")).toBeVisible();
  await page.locator("#inscription-classe").selectOption({ label: "PS · Petite section" });
  await page.getByRole("button", { name: texte("inscriptions.inscrire_et_valider") }).click();
  await expect(page.getByText(texte("inscription.photo_absente"))).toBeVisible();
  await capturer("48-jardin-refus-sans-photo");

  // La photo posée, la validation passe, avec l'avertissement d'une seule personne autorisée.
  await page.locator("#eleve-photo").setInputFiles(join(import.meta.dirname, "logo-essai.png"));
  await expect(page.getByRole("img", { name: texte("eleves.photo") })).toBeVisible();
  await page.getByRole("button", { name: texte("inscriptions.valider") }).click();
  await expect(page.getByText(`2026-2027 · PS · ${texte("inscriptions.statut.validee")}`)).toBeVisible();
  await expect(page.getByText(texte("inscription.une_seule_personne_autorisee"))).toBeVisible();
  await capturer("49-jardin-inscription-validee");
});

test("phase 3 : UC-05, les impayés par tranche d'ancienneté, puis le journal des encaissements", async () => {
  await allerA("nav.impayes");
  await expect(page.getByRole("heading", { name: texte("impayes.titre") })).toBeVisible();
  await expect(page.getByText(/élèves en retard/)).toBeVisible();
  for (const tranche of ["0 à 15 jours", "16 à 30 jours", "31 à 60 jours", "Plus de 60 jours"]) {
    await expect(page.getByRole("button", { name: new RegExp(tranche) })).toBeVisible();
  }
  // Oumar doit le reste de sa 1re tranche, échue le 5 octobre, et son inscription. L'élève du
  // jardin doit son inscription, échue aujourd'hui : 0 jour de retard.
  await expect(page.getByRole("row").filter({ hasText: "Oumar Sangaré" }).getByRole("cell", { name: "0 à 15 jours" })).toBeVisible();
  const bakary = page.getByRole("row").filter({ hasText: "Bakary Keïta" });
  await expect(bakary.getByRole("cell", { name: "0 à 15 jours" })).toBeVisible();
  await expect(bakary.getByRole("cell", { name: /Assitan Keïta \(\+223 79 00 00 01\)/ })).toBeVisible();
  await expect(page.getByRole("button", { name: /0 à 15 jours.*2 élève/ })).toBeVisible();
  await capturer("59-impayes");

  // Le journal du mois : un reçu valide de 50 000 F, et l'annulé à part.
  await allerA("nav.journal_encaissements");
  await expect(page.getByRole("status").filter({ hasText: /Reçus valides : 1 pour 50.000.F ; annulés : 1 \(10.000.F\)/ })).toBeVisible();
  await expect(page.getByRole("cell", { name: "REC-2026-000002" })).toBeVisible();
  await expect(page.getByRole("cell", { name: /Annulé : Payé en espèces/ })).toBeVisible();
  await page.locator("#journal-mode").selectOption("orange_money");
  await expect(page.getByRole("status").filter({ hasText: /Reçus valides : 0/ })).toBeVisible();
  await page.locator("#journal-mode").selectOption("");
  await expect(page.getByRole("status").filter({ hasText: /Reçus valides : 1/ })).toBeVisible();
  await capturer("60-journal-encaissements");
});

test("phase 3 : UC-01 complet, inscription, frais, premier versement et reçu sans quitter l'assistant", async () => {
  await allerA("nav.inscriptions");
  const debut = Date.now();
  await page.getByRole("button", { name: texte("inscriptions.assistant") }).click();
  await page.locator("#nouvel-eleve-matricule").fill("L/2015/021");
  await page.locator("#nouvel-eleve-nom").fill("Diallo");
  await page.locator("#nouvel-eleve-prenoms").fill("Fatoumata");
  await page.locator("#nouvel-eleve-sexe").selectOption("feminin");
  await page.locator("#nouvel-eleve-naissance").fill("2015-06-03");
  await page.locator("#nouvel-eleve-lieu_naissance").fill("Kati");
  await page.getByRole("button", { name: texte("eleves.creer") }).click();

  await page.locator("#ajout-lien").selectOption("mere");
  await page.locator("#nouveau-responsable-nom").fill("Diallo");
  await page.locator("#nouveau-responsable-prenoms").fill("Kadiatou");
  await page.locator("#nouveau-responsable-telephone-0").fill("76 45 12 90");
  await page.getByRole("button", { name: texte("responsables.creer_et_lier") }).click();
  await expect(page.getByText("+223 76 45 12 90")).toBeVisible();
  await page.getByRole("button", { name: texte("commun.continuer") }).click();

  // Classe et plan du parent : en trois tranches, celui de l'école.
  await page.locator("#inscription-classe").selectOption({ label: "7e A · 7ème année" });
  await expect(page.locator("#inscription-plan")).toBeVisible();
  await page.getByRole("button", { name: texte("inscriptions.inscrire_et_valider") }).click();
  await expect(page.getByText(`2026-2027 · 7e A · ${texte("inscriptions.statut.validee")}`)).toBeVisible();
  await page.getByRole("button", { name: texte("commun.continuer") }).click();

  // Quatrième étape : la facture et l'échéancier, puis le premier versement à la caisse ouverte.
  await expect(page.getByText(/facture FAC-2026-/)).toBeVisible();
  await expect(page.getByRole("heading", { name: texte("inscriptions.premier_versement") })).toBeVisible();
  await page.locator("#encaissement-montant").fill("10000");
  await expect(page.locator("#encaissement-payeur")).toHaveValue(/.+/);
  await page.locator("#encaissement-montant").press("Enter");
  await expect(page.getByRole("status").filter({ hasText: "REC-2026-000003" })).toBeVisible();
  const duree = Date.now() - debut;
  writeFileSync(join(CAPTURES, "uc01-complet-chronometre.txt"), `UC-01 complet : ${duree} ms, de l'ouverture de l'assistant au reçu du premier versement, piloté par Playwright (borne basse ; cible : moins de 5 minutes au clavier).\n`, "utf8");
  expect(duree).toBeLessThan(5 * 60_000);
  // Le versement apparaît sur la facture, au-dessus du reçu.
  await expect(page.getByText(/Reçu REC-2026-000003/)).toBeVisible();
  await expect(page.locator("iframe").last()).toBeVisible({ timeout: 30_000 });
  await page.waitForTimeout(4000);
  await page.getByRole("heading", { name: texte("inscriptions.premier_versement") }).scrollIntoViewIfNeeded();
  await capturer("61-uc01-premier-versement");
});

test("phase 4 : matières et coefficients d'un niveau, périodes, règles de calcul", async () => {
  await allerA("nav.pedagogie");
  await expect(page.getByRole("heading", { name: texte("pedagogie.titre") })).toBeVisible();

  // Au 1er cycle, les matières de l'emploi du temps, sans coefficient ; les domaines restent
  // disponibles, hors du programme livré.
  await page.locator("#programme-niveau").selectOption({ label: "1re année · 1er cycle" });
  await expect(page.locator("#programme-LANG")).toBeChecked();
  await expect(page.locator("#programme-LECT")).toBeChecked();
  await expect(page.locator("#programme-LANG-coefficient")).toHaveCount(0);
  await expect(page.locator("#programme-LC")).not.toBeChecked();

  // En 7e, la direction saisit les coefficients.
  await page.locator("#programme-niveau").selectOption({ label: "7ème année · 2e cycle" });
  await expect(page.locator("#programme-MATH")).toBeChecked();
  await page.locator("#programme-MATH-coefficient").fill("4");
  await page.locator("#programme-FR-coefficient").fill("4");
  await page.locator("#programme-ANG-coefficient").fill("2");
  await page.locator("#programme-PC-coefficient").fill("3");
  await page.locator("#programme-SVT-coefficient").fill("2");
  await page.locator("#programme-HG-coefficient").fill("2");
  await page.locator("#programme-ECM-coefficient").fill("1");
  await page.locator("#programme-EPS-coefficient").fill("1");
  await page.getByRole("button", { name: texte("commun.enregistrer") }).click();
  await expect(page.getByRole("status").filter({ hasText: /Enregistré à/ })).toBeVisible();
  await capturer("62-programme-coefficients");

  // Périodes : trois trimestres en 7e, deux possibles, jamais de semestre ; neuf mois au 1er cycle.
  await page.getByRole("button", { name: texte("pedagogie.onglet.periodes") }).click();
  await page.locator("#periodes-niveau").selectOption({ label: "7ème année · 2e cycle" });
  await expect(page.getByText("3e trimestre")).toBeVisible();
  await page.getByRole("button", { name: texte("periodes.deux_trimestres") }).click();
  await expect(page.getByText("2e trimestre")).toBeVisible();
  await expect(page.getByText("3e trimestre")).toHaveCount(0);
  await page.getByRole("button", { name: texte("periodes.trois_trimestres") }).click();
  await expect(page.getByText("3e trimestre")).toBeVisible();
  await page.locator("#periode-1-fin").fill("2026-12-18");
  await page.getByRole("button", { name: texte("commun.enregistrer") }).click();
  await expect(page.getByRole("status").filter({ hasText: /Enregistré à/ })).toBeVisible();
  await capturer("63-periodes");
  await page.locator("#periodes-niveau").selectOption({ label: "1re année · 1er cycle" });
  await expect(page.getByText("Juin", { exact: true })).toBeVisible();

  // Règles du 2e cycle : l'absent compte zéro ; une nouvelle version naît.
  await page.getByRole("button", { name: texte("pedagogie.onglet.regles") }).click();
  await page.locator("#regles-cycle").selectOption("second_cycle");
  await expect(page.getByText("Excellent à partir de 18")).toBeVisible();
  await expect(page.locator("#regle-mention-5-libelle")).toHaveValue("Médiocre");
  await page.locator("#regle-absent").selectOption("zero");
  await page.getByRole("button", { name: texte("commun.enregistrer") }).click();
  await expect(page.getByRole("status").filter({ hasText: "version 2" })).toBeVisible();
  await capturer("64-regles-de-calcul");
});

test("phase 4 : enseignants, maître de toute sa classe, matière confiée à un autre", async () => {
  // Une classe du 1er cycle pour le maître.
  await allerA("nav.classes");
  await page.locator("#classe-niveau").selectOption({ label: "3e année" });
  await page.locator("#classe-libelle").fill("3e A");
  await page.getByRole("button", { name: texte("classes.creer") }).click();
  await expect(page.getByRole("cell", { name: "3e A", exact: true })).toBeVisible();

  await allerA("nav.enseignants");
  await expect(page.getByRole("heading", { name: texte("enseignants.titre") })).toBeVisible();
  const enseignants: [string, string, string, string][] = [
    ["Moussa", "Diarra", "titulaire", "76 11 22 33"],
    ["Aminata", "Koné", "vacataire", "66 44 55 77"],
  ];
  for (const [prenoms, nom, statut, telephone] of enseignants) {
    await page.locator("#enseignant-prenoms").fill(prenoms);
    await page.locator("#enseignant-nom").fill(nom);
    await page.locator("#enseignant-telephone").fill(telephone);
    await page.locator("#enseignant-statut").selectOption(statut);
    await page.getByRole("button", { name: texte("commun.enregistrer") }).click();
    await expect(page.getByRole("cell", { name: prenoms, exact: true })).toBeVisible();
  }

  // Le maître reçoit toute la 3e A ; les arts vont à Aminata Koné.
  await page.locator("#affectations-classe").selectOption({ label: "3e A · 3e année" });
  await page.locator("#affectations-maitre").selectOption({ label: "Moussa Diarra" });
  await page.getByRole("button", { name: texte("enseignants.toute_la_classe") }).click();
  await expect(page.getByRole("status").filter({ hasText: "19 matières confiées." })).toBeVisible();
  await page.locator("#affectation-DESS").selectOption({ label: "Aminata Koné" });
  await page.getByRole("row").filter({ hasText: "Dessin" }).getByRole("button", { name: texte("enseignants.confier") }).click();
  await expect(page.getByRole("row").filter({ hasText: "Dessin" }).getByRole("listitem").filter({ hasText: "Aminata Koné" })).toBeVisible();
  await expect(page.getByRole("row").filter({ hasText: "Dessin" }).getByRole("listitem").filter({ hasText: "Moussa Diarra" })).toHaveCount(0);
  await expect(page.getByRole("row").filter({ hasText: "Lecture" }).getByRole("listitem").filter({ hasText: "Moussa Diarra" })).toBeVisible();
  await capturer("65-enseignants-affectations");
});

test("phase 4 : UC-03 chronométré, une composition de 90 élèves saisie au clavier", async () => {
  test.setTimeout(15 * 60_000);
  // Une classe de 90 élèves : créée, importée d'un fichier, validée d'un coup.
  await allerA("nav.classes");
  await page.locator("#classe-niveau").selectOption({ label: "7ème année" });
  await page.locator("#classe-libelle").fill("7e B");
  await page.getByRole("button", { name: texte("classes.creer") }).click();
  await expect(page.getByRole("cell", { name: "7e B", exact: true })).toBeVisible();

  const noms = ["BALLO", "CAMARA", "DOUMBIA", "FOFANA", "GUINDO", "HAIDARA", "KANTE", "MARIKO", "OUATTARA", "TANGARA"];
  const prenoms = ["Alou", "Boubou", "Daouda", "Fanta", "Habib", "Lassine", "Nana", "Salif", "Tenin"];
  const deux = (n: number) => String(n).padStart(2, "0");
  const lignes = ["N°;Nom et prénoms;Sexe;Né(e) le;Lieu de naissance;Classe;Téléphone du parent;Matricule"];
  for (let i = 0; i < 90; i++) {
    const nom = noms[Math.floor(i / 9)] ?? "BALLO";
    const prenom = prenoms[i % 9] ?? "Alou";
    const sexe = ["Fanta", "Nana", "Tenin"].includes(prenom) ? "F" : "M";
    lignes.push(`${i + 1};${nom} ${prenom};${sexe};${deux((i % 28) + 1)}/${deux((i % 12) + 1)}/2013;Bamako;7e B;70 20 ${deux(Math.floor(i / 10))} ${deux(i % 10)};B/2026/${String(i + 1).padStart(3, "0")}`);
  }
  const fichier = join(dossier, "septieme-b.csv");
  writeFileSync(fichier, `\uFEFF${lignes.join("\r\n")}\r\n`, "utf8");
  await allerA("nav.eleves");
  await page.evaluate((chemin) => {
    const fenetre = window as unknown as { fetchOriginal?: typeof fetch };
    const original = window.fetch.bind(window);
    fenetre.fetchOriginal = original;
    window.fetch = (entree, options) => {
      const url = typeof entree === "string" ? entree : entree instanceof URL ? entree.href : entree.url;
      if (decodeURIComponent(url).includes("ipc.localhost/plugin:dialog|open")) {
        return Promise.resolve(new Response(JSON.stringify(chemin), { status: 200, headers: { "Tauri-Response": "ok", "content-type": "application/json" } }));
      }
      return original(entree, options);
    };
  }, fichier);
  await page.getByRole("button", { name: texte("import.ouvrir") }).click();
  await page.getByRole("button", { name: texte("import.choisir") }).click();
  await expect(page.getByText("1 feuille(s), 90 ligne(s) lues.")).toBeVisible();
  await page.evaluate(() => {
    const fenetre = window as unknown as { fetchOriginal?: typeof fetch };
    if (fenetre.fetchOriginal !== undefined) window.fetch = fenetre.fetchOriginal;
  });
  await page.getByRole("button", { name: texte("import.verifier") }).click();
  await page.getByRole("button", { name: "Importer 90 élève(s)" }).click();
  await expect(page.getByRole("status")).toContainText("90 élève(s) créé(s)", { timeout: 60_000 });
  await page.getByRole("button", { name: texte("import.fermer") }).click();

  await allerA("nav.inscriptions");
  await page.locator("#inscriptions-classe").selectOption("7e B");
  await page.getByRole("button", { name: "Valider les 90 brouillons de la classe" }).click();
  await expect(page.getByRole("status").filter({ hasText: "90 inscriptions validées ; 0 restent en brouillon." })).toBeVisible({ timeout: 180_000 });

  // La composition de mathématiques du 1er trimestre.
  await allerA("nav.notes");
  await page.locator("#notes-classe").selectOption({ label: "7e B · 7ème année" });
  await page.locator("#notes-matiere").selectOption({ label: "Mathématiques" });
  await page.locator("#evaluation-type").selectOption("composition");
  await page.getByRole("button", { name: texte("notes.creer") }).click();
  await expect(page.locator("#note-1")).toBeFocused();

  // UC-03 : la note puis Entrée, quatre-vingt-dix fois ; quelques absents et un dispensé.
  const debut = Date.now();
  for (let i = 0; i < 90; i++) {
    const saisie = i % 23 === 5 ? "a" : i === 41 ? "aj" : i === 60 ? "d" : `${4 + ((i * 7) % 16)},${(i * 3) % 10 === 0 ? "" : String((i * 25) % 100).padStart(2, "0").replace(/0$/, "")}`.replace(/,$/, "");
    await page.keyboard.type(saisie);
    await page.keyboard.press("Enter");
    if (i < 89) await expect(page.locator(`#note-${i + 2}`)).toBeFocused();
  }
  await expect(page.getByRole("status").filter({ hasText: "90 élèves sur 90 saisis." })).toBeVisible();
  const duree = Date.now() - debut;
  test.info().annotations.push({ type: "uc03_ms", description: String(duree) });
  writeFileSync(join(CAPTURES, "uc03-chronometre.txt"), `UC-03 : ${duree} ms pour 90 élèves, de la première note à la dernière, piloté par Playwright (borne basse ; cible : moins de 10 minutes au clavier).\n`, "utf8");
  expect(duree).toBeLessThan(10 * 60_000);
  // La grille suit la liste de classe : le 6e élève, noté « a », est absent, pas à zéro.
  await expect(page.getByRole("row").filter({ has: page.locator("#note-6") }).getByText(texte("notes.absent"), { exact: true })).toBeVisible();
  await expect(page.locator("#note-6")).toHaveValue("a");
  await page.locator("#note-1").scrollIntoViewIfNeeded();
  await capturer("66-saisie-des-notes");

  // Un 21 est refusé sur place, sans quitter la ligne.
  await page.locator("#note-3").fill("21");
  await page.locator("#note-3").press("Enter");
  await expect(page.getByText(texte("note.hors_bareme"))).toBeVisible();
  await expect(page.locator("#note-3")).toBeFocused();
  await page.locator("#note-3").fill("12");
  await page.locator("#note-3").press("Enter");

  await page.getByRole("button", { name: texte("notes.valider") }).click();
  await expect(page.getByText(texte("notes.validee"))).toBeVisible();
  await expect(page.locator("#note-1")).toBeDisabled();
  await page.getByText(texte("notes.validee")).scrollIntoViewIfNeeded();
  await capturer("67-composition-validee");
});

test("phase 4 : grille Excel de l'enseignant, exportée puis relue avec son aperçu", async () => {
  await allerA("nav.notes");
  await page.locator("#notes-classe").selectOption({ label: "7e A · 7ème année" });
  await page.locator("#notes-matiere").selectOption({ label: "Mathématiques" });
  await page.locator("#evaluation-type").selectOption("devoir");
  await page.getByRole("button", { name: texte("notes.creer") }).click();
  await expect(page.locator("#note-1")).toBeVisible();
  const matricules = (await page.locator("tbody tr td:nth-child(3)").allTextContents()).map((m) => m.trim());

  // Les dialogues du système ne se pilotent pas : leurs appels reçoivent les chemins voulus.
  const exportee = join(dossier, "grille-7e-A.xlsx");
  const remplie = join(dossier, "grille-7e-A-remplie.csv");
  await page.evaluate(([cheminSave, cheminOpen]) => {
    const fenetre = window as unknown as { fetchOriginal?: typeof fetch };
    const original = window.fetch.bind(window);
    fenetre.fetchOriginal = original;
    window.fetch = (entree, options) => {
      const url = decodeURIComponent(typeof entree === "string" ? entree : entree instanceof URL ? entree.href : entree.url);
      const reponse = (chemin: string) => Promise.resolve(new Response(JSON.stringify(chemin), { status: 200, headers: { "Tauri-Response": "ok", "content-type": "application/json" } }));
      if (url.includes("ipc.localhost/plugin:dialog|save")) return reponse(cheminSave ?? "");
      if (url.includes("ipc.localhost/plugin:dialog|open")) return reponse(cheminOpen ?? "");
      return original(entree, options);
    };
  }, [exportee, remplie]);
  await page.getByRole("button", { name: texte("import_notes.exporter") }).click();
  await expect(page.getByText(/Grille enregistrée/)).toBeVisible();
  expect(existsSync(exportee)).toBe(true);

  // L'enseignant rend sa grille : une note, une note hors barème, un matricule étranger.
  const lignes = ["N°;Matricule;Élève;Note sur 20", `1;${matricules[0] ?? ""};;14,5`];
  if (matricules[1] !== undefined) lignes.push(`2;${matricules[1]};;25`);
  lignes.push("3;X/0000;Inconnu;12");
  writeFileSync(remplie, `\uFEFF${lignes.join("\r\n")}\r\n`, "utf8");
  await page.getByRole("button", { name: texte("import_notes.ouvrir") }).click();
  await expect(page.getByRole("status").filter({ hasText: /Notes à enregistrer/ })).toBeVisible();
  await expect(page.getByText(/Matricules absents de la classe, ignorés : X\/0000/)).toBeVisible();
  await page.getByRole("button", { name: "Enregistrer les notes (1)" }).scrollIntoViewIfNeeded();
  await capturer("68-import-grille-apercu");
  await page.getByRole("button", { name: "Enregistrer les notes (1)" }).click();
  await expect(page.getByText("Notes enregistrées depuis la grille : 1.")).toBeVisible();
  await expect(page.locator("#note-1")).toHaveValue("14,5");
  await page.evaluate(() => {
    const fenetre = window as unknown as { fetchOriginal?: typeof fetch };
    if (fenetre.fetchOriginal !== undefined) window.fetch = fenetre.fetchOriginal;
  });
});

test("phase 4 : relevé des absences d'une journée au clavier, puis justification", async () => {
  await allerA("nav.absences");
  await page.locator("#absences-classe").selectOption({ label: "7e A · 7ème année" });
  await expect(page.getByText(texte("absences.pas_encore_releve"))).toBeVisible();
  const premier = (await page.locator("tbody tr").first().locator("td").nth(1).textContent()) ?? "";
  const second = (await page.locator("tbody tr").nth(1).locator("td").nth(1).textContent()) ?? "";

  // Au clavier : le premier absent le matin, Entrée descend ; le second en retard d'un quart d'heure.
  await page.locator("#matin-1").fill("a");
  await page.locator("#matin-1").press("Enter");
  await expect(page.locator("#matin-2")).toBeFocused();
  await page.keyboard.type("r15");
  await page.locator("#apres-midi-1").fill("a");
  // Une case illisible bloque l'enregistrement.
  await page.locator("#apres-midi-2").fill("x");
  await expect(page.getByText("Cases illisibles : 1.", { exact: false })).toBeVisible();
  await expect(page.getByRole("button", { name: texte("absences.enregistrer") })).toBeDisabled();
  await page.locator("#apres-midi-2").fill("");
  await expect(page.getByRole("status").filter({ hasText: "Demi-journées d'absence : 2 ; retards : 1." })).toBeVisible();
  await capturer("69-releve-absences");
  await page.getByRole("button", { name: texte("absences.enregistrer") }).click();
  await expect(page.getByText("Relevé enregistré. Demi-journées d'absence : 2 ; retards : 1.")).toBeVisible();
  await expect(page.getByText(/Relevé enregistré le .* par /)).toBeVisible();

  // Le lendemain, la maman apporte un certificat : la journée du premier est justifiée.
  await page.getByRole("button", { name: texte("absences.onglet.periode") }).click();
  await expect(page.getByRole("heading", { name: "À justifier : 3" })).toBeVisible();
  const siennes = page.getByRole("checkbox", { name: new RegExp(`${premier} · Absence`) });
  await expect(siennes).toHaveCount(2);
  await siennes.nth(0).check();
  await siennes.nth(1).check();
  await page.locator("#justification-motif").fill("Maladie");
  await page.locator("#justification-piece").fill("Certificat médical");
  await page.getByRole("button", { name: "Justifier la sélection (2)" }).click();
  await expect(page.getByText("Demi-journées justifiées : 2.")).toBeVisible();
  await expect(page.getByRole("heading", { name: "À justifier : 1" })).toBeVisible();
  await expect(page.getByRole("checkbox", { name: new RegExp(`${second} · Retard de 15 min`) })).toBeVisible();
  const ligne = page.getByRole("row").filter({ hasText: premier });
  await expect(ligne.getByRole("cell")).toHaveText([premier, "2", "2", "0"]);
  await capturer("70-absences-justifiees");
});

test("phase 4 : bulletins d'une classe du 2e cycle, calculés, appréciés, validés et publiés", async () => {
  // Les notes du trimestre : le devoir de maths se complète, deux compositions s'ajoutent.
  await allerA("nav.notes");
  await page.locator("#notes-classe").selectOption({ label: "7e A · 7ème année" });
  await page.locator("#notes-matiere").selectOption({ label: "Mathématiques" });
  await page.getByRole("button", { name: /^Devoir 1/ }).click();
  await page.locator("#note-2").fill("a");
  await page.locator("#note-2").press("Enter");
  await page.getByRole("button", { name: texte("notes.valider") }).click();
  await expect(page.getByText(texte("notes.validee"))).toBeVisible();
  await page.getByRole("button", { name: texte("notes.retour") }).click();
  const composer = async (matiere: string, notes: readonly string[]) => {
    await page.locator("#notes-matiere").selectOption({ label: matiere });
    await page.locator("#evaluation-type").selectOption("composition");
    await page.getByRole("button", { name: texte("notes.creer") }).click();
    for (const [i, note] of notes.entries()) {
      await page.locator(`#note-${i + 1}`).fill(note);
      await page.locator(`#note-${i + 1}`).press("Enter");
    }
    await expect(page.getByRole("status").filter({ hasText: `${notes.length} élèves sur ${notes.length}` })).toBeVisible();
    await page.getByRole("button", { name: texte("notes.valider") }).click();
    await expect(page.getByText(texte("notes.validee"))).toBeVisible();
    await page.getByRole("button", { name: texte("notes.retour") }).click();
  };
  await composer("Mathématiques", ["12", "15"]);
  await composer("Français", ["13", "16"]);

  await allerA("nav.bulletins");
  await page.locator("#bulletins-classe").selectOption({ label: "7e A · 7ème année" });
  await page.getByRole("button", { name: texte("bulletins.calculer") }).click();
  await expect(page.getByText("Bulletins calculés : 2.")).toBeVisible();
  const premiere = page.getByRole("row").filter({ hasText: "Oumar Sangaré" });
  await expect(premiere.getByRole("cell").first()).toHaveText("1er");
  await page.getByRole("row").filter({ hasText: "Fatoumata Diallo" }).getByRole("button", { name: texte("bulletins.ouvrir") }).click();
  const maths = page.getByLabel("Appréciation en Mathématiques");
  await maths.fill("Bon travail");
  await maths.press("Enter");
  await page.locator("#appreciation-generale").fill("Élève sérieuse ; doit soigner les maths.");
  await page.locator("#appreciation-generale").press("Tab");
  await expect(page.getByText(/Moyenne générale : .* rang 2e sur 2/)).toBeVisible();
  await capturer("71-bulletins-classe");

  await page.getByRole("button", { name: texte("bulletins.valider") }).click();
  await expect(page.getByText("Bulletins validés : 2.")).toBeVisible();
  await page.getByRole("button", { name: texte("bulletins.publier") }).click();
  await expect(page.getByText(texte("bulletins.publies"))).toBeVisible();
  await expect(page.getByRole("row").filter({ hasText: "Fatoumata Diallo" }).getByText(texte("bulletins.statut.publie"))).toBeVisible();
  await page.getByRole("row").filter({ hasText: "Fatoumata Diallo" }).getByRole("button", { name: texte("bulletins.fermer") }).click();
  await expect(page.locator("iframe")).toBeVisible({ timeout: 30_000 });
  // Le visionneur PDF de WebView2 rend la page hors du DOM : aucun signal à attendre.
  await page.waitForTimeout(4000);
  await page.locator("iframe").scrollIntoViewIfNeeded();
  await capturer("72-bulletin-publie-pdf");
});

test("phase 4 : l'école choisit les colonnes du bulletin imprimé", async () => {
  await allerA("nav.parametres");
  // Les bulletins du 2e cycle seulement : chaque cycle a ses propres réglages.
  const bloc = page.locator("section").filter({ has: page.getByRole("heading", { name: texte("parametres.theme.bulletin") }) });
  await bloc.locator("#bulletin-cycle").selectOption("second_cycle");
  for (const cle of ["colonne.coefficient", "colonne.points", "colonne.appreciation"]) {
    await bloc.locator(`[id="bulletin-${cle}"]`).uncheck();
  }
  await bloc.getByRole("button", { name: texte("commun.enregistrer") }).click();
  await expect(bloc.getByText(/Enregistré/)).toBeVisible();
  // Le 1er cycle garde son appréciation.
  await bloc.locator("#bulletin-cycle").selectOption("premier_cycle");
  await expect(bloc.locator('[id="bulletin-colonne.appreciation"]')).toBeChecked();
  await expect(bloc.locator('[id="bulletin-colonne.coefficient"]')).toHaveCount(0);
  await bloc.locator("#bulletin-cycle").selectOption("second_cycle");
  await page.getByRole("heading", { name: texte("parametres.theme.bulletin") }).scrollIntoViewIfNeeded();
  await capturer("73-bulletin-colonnes-choisies");

  await allerA("nav.bulletins");
  await page.locator("#bulletins-classe").selectOption({ label: "7e A · 7ème année" });
  await page.getByRole("button", { name: texte("bulletins.apercu") }).click();
  await expect(page.locator("iframe")).toBeVisible({ timeout: 30_000 });
  // Le visionneur PDF de WebView2 rend la page hors du DOM : aucun signal à attendre.
  await page.waitForTimeout(4000);
  await page.locator("iframe").scrollIntoViewIfNeeded();
  await capturer("74-bulletin-sans-coefficients");
});

test("phase 4 : moyenne annuelle de la 7e A au 3e trimestre, enregistrée sans être imprimée", async () => {
  // Une composition de maths aux 2e et 3e trimestres, puis les bulletins de chacun.
  for (const [periode, date, notes] of [
    ["2e trimestre", "2027-02-10", ["13", "11"]],
    ["3e trimestre", "2027-05-10", ["14", "12"]],
  ] as const) {
    await allerA("nav.notes");
    await page.locator("#notes-classe").selectOption({ label: "7e A · 7ème année" });
    await page.locator("#notes-periode").selectOption({ label: periode });
    await page.locator("#notes-matiere").selectOption({ label: "Mathématiques" });
    await page.locator("#evaluation-type").selectOption("composition");
    await page.locator("#evaluation-date").fill(date);
    await page.getByRole("button", { name: texte("notes.creer") }).click();
    for (const [i, note] of notes.entries()) {
      await page.locator(`#note-${i + 1}`).fill(note);
      await page.locator(`#note-${i + 1}`).press("Enter");
    }
    await expect(page.getByRole("status").filter({ hasText: "2 élèves sur 2" })).toBeVisible();
    await page.getByRole("button", { name: texte("notes.valider") }).click();
    await expect(page.getByText(texte("notes.validee"))).toBeVisible();

    await allerA("nav.bulletins");
    await page.locator("#bulletins-classe").selectOption({ label: "7e A · 7ème année" });
    await page.locator("#bulletins-periode").selectOption({ label: periode });
    await page.getByRole("button", { name: texte("bulletins.calculer") }).click();
    await expect(page.getByText("Bulletins calculés : 2.")).toBeVisible();
    await page.getByRole("button", { name: texte("bulletins.valider") }).click();
    await expect(page.getByText("Bulletins validés : 2.")).toBeVisible();
    await page.getByRole("button", { name: texte("bulletins.publier") }).click();
    await expect(page.getByText(texte("bulletins.publies"))).toBeVisible();
  }
  await page.getByRole("row").filter({ hasText: "Fatoumata Diallo" }).getByRole("button", { name: texte("bulletins.ouvrir") }).click();
  await expect(page.getByText(/Moyenne annuelle : .*, rang /)).toBeVisible();
  await expect(page.getByText(texte("bulletins.annuelle_enregistree"))).toBeVisible();
  await page.getByText(/Moyenne annuelle/).scrollIntoViewIfNeeded();
  await capturer("75-moyenne-annuelle");
});

test("phase 4 : exclusion temporaire, puis transfert vers une autre école et son certificat", async () => {
  const jour = (decalage: number) => new Date(Date.parse(JOUR_ESSAI) + decalage * 86_400_000).toISOString().slice(0, 10);

  // Fatoumata est exclue trois jours : elle reste inscrite, et le relevé ne la compte pas absente.
  await allerA("nav.eleves");
  await page.locator("#eleves-recherche").fill("Fatoumata");
  await page.getByRole("cell", { name: "Fatoumata" }).first().click();
  await expect(page.getByRole("heading", { name: "Fatoumata Diallo" })).toBeVisible();
  await page.getByText(texte("inscriptions.autres_actions")).click();
  await page.getByRole("button", { name: texte("depart.exclure") }).click();
  await page.locator("[id^=exclusion-du-]").fill(jour(0));
  await page.locator("[id^=exclusion-au-]").fill(jour(2));
  await page.locator("[id^=exclusion-motif-]").fill("Insolence envers un surveillant");
  await page.locator("[id^=exclusion-decision-]").fill("Décision du directeur");
  await page.getByRole("button", { name: texte("exclusions.confirmer") }).click();
  const exclusion = page.getByRole("listitem").filter({ hasText: /Exclusion temporaire du .* : Insolence envers un surveillant/ }).last();
  await expect(exclusion).toBeVisible();
  await expect(exclusion.getByText(texte("exclusions.statut.en_cours"))).toBeVisible();
  await exclusion.scrollIntoViewIfNeeded();
  await capturer("76-exclusion-temporaire");

  await allerA("nav.absences");
  await page.locator("#absences-classe").selectOption({ label: "7e A · 7ème année" });
  const ligneFatoumata = page.getByRole("row").filter({ hasText: "Fatoumata Diallo" });
  await expect(ligneFatoumata.getByText(texte("absences.exclu"))).toBeVisible();
  await expect(ligneFatoumata.getByRole("textbox").first()).toBeDisabled();

  // Oumar part pour le collège de Kati : certificat de transfert numéroté.
  await allerA("nav.eleves");
  await page.locator("#eleves-recherche").fill("Oumar");
  await page.getByRole("cell", { name: "Oumar" }).click();
  await expect(page.getByRole("heading", { name: "Oumar Sangaré" })).toBeVisible();
  await page.getByText(texte("inscriptions.autres_actions")).click();
  await page.getByRole("button", { name: texte("depart.transferer") }).click();
  await page.locator("[id^=depart-destination-]").fill("Collège de Kati");
  await page.locator("[id^=depart-motif-]").fill("Déménagement de la famille");
  await page.getByRole("button", { name: texte("depart.confirmer_transfert") }).click();
  await expect(page.getByText(/Transfert enregistré\. Certificat CTR-2026-000001/)).toBeVisible();
  await expect(page.getByText(/· Transférée$/)).toBeVisible();
  await page.getByRole("button", { name: texte("depart.certificat") }).click();
  await expect(page.locator("iframe")).toBeVisible({ timeout: 30_000 });
  // Le visionneur PDF de WebView2 rend la page hors du DOM : aucun signal à attendre.
  await page.waitForTimeout(4000);
  await page.locator("iframe").scrollIntoViewIfNeeded();
  await capturer("77-certificat-de-transfert");
});

test("phase 4 : UC-04 au 1er cycle, bulletins mensuels de la 3e A sur 10, chronométré", async () => {
  // Deux élèves en 3e A, inscrits par l'assistant.
  const eleves: [string, string, string, string, string][] = [
    ["Aïcha", "Coulibaly", "L/2019/031", "feminin", "76 31 00 01"],
    ["Bakary", "Keïta", "L/2019/032", "masculin", "76 31 00 02"],
  ];
  for (const [prenoms, nom, matricule, sexe, telephone] of eleves) {
    await allerA("nav.inscriptions");
    await page.getByRole("button", { name: texte("inscriptions.assistant") }).click();
    await page.locator("#nouvel-eleve-matricule").fill(matricule);
    await page.locator("#nouvel-eleve-nom").fill(nom);
    await page.locator("#nouvel-eleve-prenoms").fill(prenoms);
    await page.locator("#nouvel-eleve-sexe").selectOption(sexe);
    await page.locator("#nouvel-eleve-naissance").fill("2018-02-11");
    await page.locator("#nouvel-eleve-lieu_naissance").fill("Bamako");
    await page.getByRole("button", { name: texte("eleves.creer") }).click();
    await page.locator("#ajout-lien").selectOption("mere");
    await page.locator("#nouveau-responsable-nom").fill(nom);
    await page.locator("#nouveau-responsable-prenoms").fill("Mariam");
    await page.locator("#nouveau-responsable-telephone-0").fill(telephone);
    await page.getByRole("button", { name: texte("responsables.creer_et_lier") }).click();
    await page.getByRole("button", { name: texte("commun.continuer") }).click();
    await page.locator("#inscription-classe").selectOption({ label: "3e A · 3e année" });
    await page.getByRole("button", { name: texte("inscriptions.inscrire_et_valider") }).click();
    await expect(page.getByText(`2026-2027 · 3e A · ${texte("inscriptions.statut.validee")}`)).toBeVisible();
  }

  // UC-04 : la composition d'octobre de cinq matières, sur 10, puis les bulletins du mois ; les
  // autres matières, sans note, sortent de la moyenne.
  const debut = Date.now();
  const notes: Record<string, string[]> = { "Aïcha Coulibaly": ["7,5", "6", "8", "9", "7"], "Bakary Keïta": ["5", "6", "4", "7", "6"] };
  await allerA("nav.notes");
  await page.locator("#notes-classe").selectOption({ label: "3e A · 3e année" });
  const toutes = await page.locator("#notes-matiere option").allTextContents();
  expect(toutes).toHaveLength(19);
  const domaines = toutes.slice(0, 5);
  for (const [d, domaine] of domaines.entries()) {
    await page.locator("#notes-matiere").selectOption({ label: domaine });
    await page.getByRole("button", { name: texte("notes.creer") }).click();
    for (let i = 0; i < 2; i += 1) {
      const eleve = ((await page.locator("tbody tr").nth(i).locator("td").nth(1).textContent()) ?? "").trim();
      await page.locator(`#note-${i + 1}`).fill(notes[eleve]?.[d] ?? "");
      await page.locator(`#note-${i + 1}`).press("Enter");
    }
    await expect(page.getByRole("status").filter({ hasText: "2 élèves sur 2" })).toBeVisible();
    await page.getByRole("button", { name: texte("notes.valider") }).click();
    await expect(page.getByText(texte("notes.validee"))).toBeVisible();
    await page.getByRole("button", { name: texte("notes.retour") }).click();
  }
  await allerA("nav.bulletins");
  await page.locator("#bulletins-classe").selectOption({ label: "3e A · 3e année" });
  await expect(page.locator("#bulletins-periode")).toHaveValue(/.+/);
  await page.getByRole("button", { name: texte("bulletins.calculer") }).click();
  await expect(page.getByText("Bulletins calculés : 2.")).toBeVisible();
  // (7,5 + 6 + 8 + 9 + 7) ÷ 5 = 7,50, « Bien ».
  const aicha = page.getByRole("row").filter({ hasText: "Aïcha Coulibaly" });
  await expect(aicha.getByRole("cell")).toHaveText(["1er", "Aïcha Coulibaly", "7,50", "Bien", texte("bulletins.statut.calcule"), texte("bulletins.ouvrir")]);
  await page.getByRole("button", { name: texte("bulletins.valider") }).click();
  await expect(page.getByText("Bulletins validés : 2.")).toBeVisible();
  await page.getByRole("button", { name: texte("bulletins.publier") }).click();
  await expect(page.getByText(texte("bulletins.publies"))).toBeVisible();
  const duree = Date.now() - debut;
  writeFileSync(join(CAPTURES, "uc04-premier-cycle-chronometre.txt"), `UC-04 au 1er cycle : ${duree} ms, des cinq compositions d'octobre saisies au PDF des bulletins publiés de la 3e A (2 élèves), piloté par Playwright.\n`, "utf8");
  await expect(page.locator("iframe")).toBeVisible({ timeout: 30_000 });
  // Le visionneur PDF de WebView2 rend la page hors du DOM : aucun signal à attendre.
  await page.waitForTimeout(4000);
  await page.locator("iframe").scrollIntoViewIfNeeded();
  await capturer("78-uc04-bulletin-premier-cycle");
});

test("tableau de bord : effectifs, finances et scolarité de l'école remplie", async () => {
  await allerA("nav.accueil");
  await expect(page.getByRole("heading", { name: texte("tableau.effectifs") })).toBeVisible();
  await expect(page.getByRole("heading", { name: texte("tableau.finances") })).toBeVisible();
  await expect(page.getByRole("heading", { name: texte("tableau.scolarite") })).toBeVisible();
  await expect(page.getByRole("row").filter({ hasText: "7e B" })).toBeVisible();
  await capturer("79-tableau-de-bord");
});

test("phase 6 : registre du personnel, fonctions de l'école et dossier", async () => {
  await allerA("nav.personnel");
  await expect(page.getByRole("heading", { name: texte("personnel.titre") })).toBeVisible();
  // Les enseignants des phases précédentes y sont déjà, sans ressaisie.
  await expect(page.getByRole("row").filter({ hasText: texte("personnel.present_enseignant") }).first()).toBeVisible();

  // Une cuisinière payée par mobile money.
  await page.getByRole("button", { name: texte("personnel.nouveau") }).click();
  await page.locator("#membre-prenoms").fill("Kadiatou");
  await page.locator("#membre-nom").fill("Diarra");
  await page.locator("#membre-sexe").selectOption("feminin");
  await page.locator("#membre-telephone").fill("76 45 12 30");
  await page.locator("#membre-categorie").selectOption({ label: "Appui" });
  await page.locator("#membre-fonction").fill("Cuisinière");
  await page.locator("#membre-arrivee").fill("2024-10-01");
  await page.locator("#membre-paiement").selectOption("mobile_money");
  await page.locator("#membre-paiement-numero").fill("66 12 34 56");
  await page.getByRole("button", { name: texte("commun.enregistrer"), exact: true }).click();
  await expect(page.getByRole("heading", { name: /Kadiatou Diarra · P-\d{4}/ })).toBeVisible();

  // Un fonctionnaire de l'État qui vient donner des heures, avec son diplôme.
  await page.getByRole("button", { name: texte("personnel.nouveau") }).click();
  await page.locator("#membre-prenoms").fill("Bakary");
  await page.locator("#membre-nom").fill("Sangaré");
  await page.locator("#membre-sexe").selectOption("masculin");
  await page.locator("#membre-telephone").fill("70 20 30 40");
  await page.locator("#membre-categorie").selectOption({ label: "Enseignement" });
  await page.locator("#membre-fonction").fill("Professeur de mathématiques");
  await page.locator("#membre-fonctionnaire").check();
  await page.locator("#membre-numero-fonctionnaire").fill("0123456-A");
  await page.getByRole("button", { name: texte("personnel.diplome.ajouter") }).click();
  await page.locator("#diplome-0-intitule").fill("Licence de mathématiques");
  await page.locator("#diplome-0-delivre").fill("Université de Bamako");
  await page.locator("#diplome-0-annee").fill("2008");
  await page.getByRole("button", { name: texte("commun.enregistrer"), exact: true }).click();
  await expect(page.getByRole("heading", { name: /Bakary Sangaré · P-\d{4}/ })).toBeVisible();

  // Son diplôme scanné rejoint le dossier ; le dialogue du système reçoit le chemin voulu.
  const scan = join(dossier, "licence-sangare.pdf");
  writeFileSync(scan, "%PDF-1.4 licence de Bakary Sangaré");
  await page.evaluate((chemin) => {
    const fenetre = window as unknown as { fetchOriginal?: typeof fetch };
    const original = window.fetch.bind(window);
    fenetre.fetchOriginal = original;
    window.fetch = (entree, options) => {
      const url = decodeURIComponent(typeof entree === "string" ? entree : entree instanceof URL ? entree.href : entree.url);
      if (url.includes("ipc.localhost/plugin:dialog|open")) {
        return Promise.resolve(new Response(JSON.stringify(chemin), { status: 200, headers: { "Tauri-Response": "ok", "content-type": "application/json" } }));
      }
      return original(entree, options);
    };
  }, scan);
  await page.locator("#piece-intitule").fill("Licence de mathématiques");
  await page.locator("#piece-nature").selectOption("diplome");
  await page.getByRole("button", { name: texte("personnel.piece.ajouter") }).click();
  await expect(page.getByText("Pièce ajoutée : Licence de mathématiques.")).toBeVisible();
  await page.evaluate(() => {
    const fenetre = window as unknown as { fetchOriginal?: typeof fetch };
    if (fenetre.fetchOriginal !== undefined) window.fetch = fenetre.fetchOriginal;
  });

  // Il devient enseignant vacataire d'un clic.
  await page.getByText(texte("personnel.autres_actions")).click();
  await page.locator("#membre-statut-enseignant").selectOption("vacataire");
  await page.getByRole("button", { name: texte("personnel.devenir_enseignant") }).click();
  await expect(page.getByRole("row").filter({ hasText: "Sangaré" }).filter({ hasText: texte("personnel.present_enseignant") })).toBeVisible();
  await capturer("80-fiche-du-personnel");

  // L'école ajoute sa propre fonction.
  await page.getByText(texte("personnel.categories_fonctions")).click();
  await page.locator("#fonction-categorie").selectOption({ label: "Appui" });
  await page.locator("#fonction-nouvelle").fill("Veilleur de nuit");
  await page.getByRole("button", { name: texte("commun.ajouter"), exact: true }).last().click();
  await expect(page.getByText("Veilleur de nuit", { exact: true })).toBeVisible();
  await page.locator("#personnel-recherche").scrollIntoViewIfNeeded();
  await capturer("81-registre-du-personnel");
});

test("phase 6 : engagements, vacation d'un fonctionnaire et sa lettre, durée déterminée qui finit bientôt", async () => {
  await allerA("nav.personnel");
  await page.getByRole("cell", { name: "Bakary Sangaré" }).click();
  await expect(page.getByRole("heading", { name: texte("engagement.titre") })).toBeVisible();

  // Le fonctionnaire est engagé en vacation : taux de l'heure, six heures par semaine, jusqu'à
  // la fin de l'année scolaire.
  await page.getByRole("button", { name: texte("engagement.nouveau") }).click();
  await page.locator("#engagement-nature").selectOption("vacation");
  await page.locator("#engagement-taux").fill("2500");
  await page.locator("#engagement-heures").fill("6");
  await page.locator("#engagement-note").fill("Mathématiques en 11S et 12TSE");
  await page.getByRole("button", { name: texte("engagement.conclure") }).click();
  const vacation = page.getByRole("listitem").filter({ hasText: texte("engagement.nature.vacation") });
  await expect(vacation.getByText(/2.500.F l'heure · 6 h par semaine/)).toBeVisible();
  await expect(vacation.getByText(texte("engagement.statut.en_cours"))).toBeVisible();
  await vacation.getByRole("button", { name: texte("engagement.lettre") }).click();
  // Le visionneur PDF de WebView2 rend la page hors du DOM : aucun signal à attendre.
  await page.waitForTimeout(3000);
  await page.locator("iframe").scrollIntoViewIfNeeded();
  await capturer("82-engagement-de-vacation");

  // La cuisinière, à durée déterminée jusqu'au 30 novembre : la fin proche est signalée.
  await allerA("nav.personnel");
  await page.getByRole("cell", { name: "Kadiatou Diarra" }).click();
  await page.getByRole("button", { name: texte("engagement.nouveau") }).click();
  await page.locator("#engagement-nature").selectOption("duree_determinee");
  await page.locator("#engagement-debut").fill("2026-06-01");
  await page.locator("#engagement-fin").fill("2026-11-30");
  await page.locator("#engagement-montant").fill("45 000");
  await page.getByRole("button", { name: texte("engagement.conclure") }).click();
  const cdd = page.getByRole("listitem").filter({ hasText: texte("engagement.nature.duree_determinee") });
  await expect(cdd.getByText(texte("engagement.fin_proche"))).toBeVisible();
  await expect(cdd.getByText(/45.000.F par mois/)).toBeVisible();
  // Renouvelée de six mois : GESCO propose la période suivante, à confirmer.
  await cdd.getByRole("button", { name: texte("engagement.renouveler") }).click();
  await expect(page.locator("#engagement-debut")).toHaveValue("2026-12-01");
  await expect(page.locator("#engagement-fin")).toHaveValue("2027-05-31");
  await page.getByRole("button", { name: texte("engagement.conclure") }).click();
  await expect(page.getByRole("listitem").filter({ hasText: texte("engagement.nature.duree_determinee") })).toHaveCount(2);
  // Renouvelée, elle n'est plus signalée.
  await expect(page.getByText(texte("engagement.renouvele"))).toBeVisible();
  await expect(page.getByText(texte("engagement.fin_proche"))).toHaveCount(0);
  await capturer("83-engagements-renouveles");
});

test("phase 6 : heures des vacataires du mois, saisies au clavier, visées puis imprimées", async () => {
  await allerA("nav.heures");
  await expect(page.locator("#heures-mois")).toHaveValue("2026-10");
  // La vacation du fonctionnaire : 6 heures par semaine, soit 24 heures prévues.
  const ligne = page.getByRole("row").filter({ hasText: "Bakary Sangaré" });
  await expect(ligne.getByRole("textbox", { name: /Heures prévues/ })).toHaveValue("24");
  await ligne.getByRole("textbox", { name: /Observation/ }).fill("Cours du 21 manqué, rattrapé le 28 pour moitié");
  await ligne.getByRole("textbox", { name: /Heures faites/ }).fill("22,5");
  await ligne.getByRole("textbox", { name: /Heures faites/ }).press("Enter");
  // Le montant se recalcule après l'enregistrement de la ligne : sur un poste chargé, plus de cinq
  // secondes.
  await expect(ligne.getByRole("cell", { name: /56.250.F/ })).toBeVisible({ timeout: 15_000 });
  await page.getByRole("button", { name: texte("heures.viser") }).click();
  await expect(page.getByRole("status").filter({ hasText: texte("heures.mois_vise") })).toBeVisible();
  await expect(ligne.getByText(texte("heures.visee"))).toBeVisible();
  await expect(ligne.getByRole("textbox", { name: /Heures faites/ })).toBeDisabled();
  await capturer("84-heures-des-vacataires");
  await page.getByRole("button", { name: texte("heures.imprimer") }).click();
  await expect(page.locator("iframe")).toBeVisible();
});

test("phase 6 : une permission demandée puis accordée, un retard noté, une absence refusée pendant la permission", async () => {
  await allerA("nav.conges");
  await expect(page.locator("#conges-mois")).toHaveValue("2026-10");
  // La cuisinière a eu deux jours de permission, notés après coup.
  await page.locator("#conge-membre").selectOption({ label: "Kadiatou Diarra · Cuisinière" });
  await page.locator("#conge-type").selectOption({ label: "Permission d'absence" });
  await page.locator("#conge-debut").fill("2026-10-12");
  await page.locator("#conge-fin").fill("2026-10-13");
  await page.locator("#conge-motif").fill("Baptême de son neveu à Ségou");
  await expect(page.getByText(/^2 jour\(s\) comptés, du lundi au vendredi/)).toBeVisible();
  await page.getByRole("button", { name: texte("conges.enregistrer_demande") }).click();
  const demande = page.getByRole("row").filter({ hasText: "Kadiatou Diarra" }).filter({ hasText: texte("conges.statut.demande") });
  await expect(demande).toBeVisible();
  await demande.getByRole("button", { name: texte("conges.accorder") }).click();
  await expect(page.getByRole("row").filter({ hasText: "Kadiatou Diarra" }).getByText(texte("conges.statut.accorde"), { exact: true })).toBeVisible();

  // Le professeur vacataire est arrivé en retard ce matin.
  await page.locator("#absence-membre").selectOption({ label: "Bakary Sangaré · Professeur de mathématiques" });
  await page.locator("#absence-nature").selectOption("retard");
  await page.locator("#absence-minutes").fill("25");
  await page.locator("#absence-motif").fill("Panne de moto");
  await page.getByRole("button", { name: texte("conges.noter"), exact: true }).click();
  await expect(page.getByRole("row").filter({ hasText: "Bakary Sangaré" }).getByText("Retard de 25 min")).toBeVisible();

  // Une absence de la cuisinière pendant sa permission est refusée : la permission suffit.
  await page.locator("#absence-membre").selectOption({ label: "Kadiatou Diarra · Cuisinière" });
  await page.locator("#absence-date").fill("2026-10-13");
  await page.locator("#absence-nature").selectOption("journee");
  await page.getByRole("button", { name: texte("conges.noter"), exact: true }).click();
  await expect(page.getByText(texte("absence_personnel.en_conge"))).toBeVisible();
  await capturer("85-absences-et-conges");
});

/** Ouvre la session d'un compte neuf depuis l'écran verrouillé, mot de passe initial changé. */
async function premiereConnexion(identifiant: string, initial: string, nouveau: string) {
  await page.getByRole("button", { name: texte("bandeau.verrouiller") }).click();
  await page.getByRole("button", { name: texte("verrou.autre_utilisateur") }).click();
  await page.getByLabel(texte("connexion.identifiant")).fill(identifiant);
  await page.getByLabel(texte("connexion.mot_de_passe")).fill(initial);
  await page.getByRole("button", { name: texte("connexion.valider") }).click();
  const changement = page.locator("form").first();
  await changement.getByLabel(texte("compte.mot_de_passe_actuel")).fill(initial);
  await changement.getByLabel(texte("compte.nouveau_mot_de_passe")).fill(nouveau);
  await changement.getByRole("button", { name: texte("commun.enregistrer") }).click();
  await expect(page.getByText(texte("compte.pin_absent"))).toBeVisible();
}

test("phase 6 : la secrétaire lit les fiches sans les montants, le comptable les voit, chacune lit la sienne ; le personnel au tableau de bord", async () => {
  // La direction crée le compte de la secrétaire et celui du comptable…
  await allerA("nav.utilisateurs");
  for (const [identifiant, nom, initial, role] of [
    ["mariam", "Mariam Keïta", "bureau-2026-initial", "secretaire"],
    ["seydou", "Seydou Dembélé", "comptes-2026-initial", "comptable"],
  ] as const) {
    await page.locator("#nouvel-identifiant").fill(identifiant);
    await page.locator("#nouveau-nom").fill(nom);
    await page.locator("#nouveau-mdp-initial").fill(initial);
    await page.locator(`#nouveau-role-${role}`).check();
    await page.getByRole("button", { name: texte("utilisateurs.creer") }).click();
    await expect(page.getByRole("cell", { name: nom })).toBeVisible();
  }

  // … puis la fiche de la secrétaire, liée à son compte, et son engagement jusqu'au 11 décembre.
  await allerA("nav.personnel");
  await page.getByRole("button", { name: texte("personnel.nouveau") }).click();
  await page.locator("#membre-prenoms").fill("Mariam");
  await page.locator("#membre-nom").fill("Keïta");
  await page.locator("#membre-sexe").selectOption("feminin");
  await page.locator("#membre-categorie").selectOption({ label: "Direction et administration" });
  await page.locator("#membre-fonction").fill("Secrétaire");
  await page.locator("#membre-arrivee").fill("2025-10-01");
  await page.locator("#membre-compte").selectOption({ label: "Mariam Keïta" });
  await page.getByRole("button", { name: texte("commun.enregistrer"), exact: true }).click();
  await expect(page.getByRole("heading", { name: /Mariam Keïta · P-\d{4}/ })).toBeVisible();
  await page.getByRole("button", { name: texte("engagement.nouveau") }).click();
  await page.locator("#engagement-nature").selectOption("duree_determinee");
  await page.locator("#engagement-debut").fill("2026-09-01");
  await page.locator("#engagement-fin").fill("2026-12-11");
  await page.locator("#engagement-montant").fill("90 000");
  await page.getByRole("button", { name: texte("engagement.conclure") }).click();
  await expect(page.getByText(/90.000.F par mois/)).toBeVisible();

  // La secrétaire enregistre sa demande de congé ; la décision revient à la direction.
  await premiereConnexion("mariam", "bureau-2026-initial", "bureau-rentree-2026");
  await allerA("nav.conges");
  await page.locator("#conge-membre").selectOption({ label: "Mariam Keïta · Secrétaire" });
  await page.locator("#conge-type").selectOption({ label: "Congé annuel" });
  await page.locator("#conge-debut").fill("2026-11-02");
  await page.locator("#conge-fin").fill("2026-11-06");
  await page.locator("#conge-motif").fill("Mariage de sa sœur à Kayes");
  await page.getByRole("button", { name: texte("conges.enregistrer_demande") }).click();
  const demande = page.getByRole("row").filter({ hasText: "Mariam Keïta" }).filter({ hasText: texte("conges.statut.demande") });
  await expect(demande).toBeVisible();
  await expect(demande.getByRole("button", { name: texte("conges.accorder") })).toHaveCount(0);

  // La fiche de la cuisinière : ses engagements, sans aucun montant.
  await allerA("nav.personnel");
  await page.getByRole("cell", { name: "Kadiatou Diarra" }).click();
  await expect(page.getByText(texte("engagement.montants_reserves"))).toBeVisible();
  await expect(page.getByRole("listitem").filter({ hasText: texte("engagement.nature.duree_determinee") })).toHaveCount(2);
  await expect(page.getByText(/F par mois/)).toHaveCount(0);
  await expect(page.getByRole("button", { name: texte("engagement.nouveau") })).toHaveCount(0);
  await expect(page.getByRole("button", { name: texte("engagement.lettre") })).toHaveCount(0);
  await capturer("86-fiche-sans-montants");

  // Sa propre fiche, dans « Mon compte » : son engagement, montant compris.
  await allerA("nav.compte");
  await expect(page.getByRole("heading", { name: texte("compte.ma_fiche") })).toBeVisible();
  await expect(page.getByText(/90.000.F par mois/)).toBeVisible();
  await capturer("87-ma-fiche");

  // Au tableau de bord : l'engagement qui finit bientôt, le retard du matin, la demande en attente.
  await allerA("nav.accueil");
  await expect(page.getByRole("heading", { name: texte("tableau.personnel") })).toBeVisible();
  await expect(page.getByText(texte("tableau.echeance").replace("{nature}", texte("engagement.nature.duree_determinee")).replace("{fin}", "11 décembre 2026"))).toBeVisible();
  await expect(page.getByText("Retard de 25 min")).toBeVisible();
  await page.getByRole("heading", { name: texte("tableau.personnel") }).scrollIntoViewIfNeeded();
  await capturer("88-tableau-de-bord-personnel");

  // Le comptable lit les montants, sans conclure d'engagement ni viser d'heures.
  await premiereConnexion("seydou", "comptes-2026-initial", "comptes-rentree-2026");
  await allerA("nav.personnel");
  await page.getByRole("cell", { name: "Kadiatou Diarra" }).click();
  await expect(page.getByText(/45.000.F par mois/).first()).toBeVisible();
  await expect(page.getByRole("button", { name: texte("engagement.nouveau") })).toHaveCount(0);
  await expect(page.getByRole("button", { name: texte("engagement.lettre") }).first()).toBeVisible();
  await capturer("89-fiche-du-comptable");
  await allerA("nav.heures");
  const ligne = page.getByRole("row").filter({ hasText: "Bakary Sangaré" });
  await expect(ligne.getByRole("cell", { name: /56.250.F/ })).toBeVisible();
  await expect(ligne.getByRole("textbox", { name: /Heures faites/ })).toBeDisabled();
  await expect(page.getByRole("button", { name: texte("heures.viser") })).toHaveCount(0);

  // La direction reprend le poste.
  await page.getByRole("button", { name: texte("bandeau.verrouiller") }).click();
  await page.getByRole("button", { name: /Awa Traoré/ }).click();
  await page.getByLabel(texte("verrou.pin")).fill(PIN);
  await page.getByRole("button", { name: texte("verrou.deverrouiller") }).click();
  await expect(page.getByText(texte("bandeau.utilisateur").replace("{nom}", NOM))).toBeVisible();
});

test("phase 6 : l'école ouvre l'enseignement technique, crée sa filière Électricité, son programme et ses semestres", async () => {
  // Les années du CAP et du BT s'ouvrent avec le cycle ; l'école crée ses filières.
  await allerA("nav.niveaux");
  await page.locator("#cycle-technique").check();
  await page.locator("form").filter({ has: page.locator("#cycle-technique") }).getByRole("button", { name: texte("commun.enregistrer") }).click();
  await expect(page.getByRole("heading", { name: texte("niveaux.filieres") })).toBeVisible();
  await page.locator("#filiere-code").fill("ELEC");
  await page.locator("#filiere-libelle").fill("Électricité");
  await page.locator("#filiere-niveau-CAP1").check();
  await page.locator("#filiere-niveau-CAP2").check();
  await page.getByRole("button", { name: texte("niveaux.filiere_creer") }).click();
  await expect(page.getByRole("row").filter({ hasText: "ELEC" })).toHaveCount(2);
  await page.locator("#filiere-code").scrollIntoViewIfNeeded();
  await capturer("90-cycle-technique-et-filiere");

  // Le programme du CAP 1 Électricité : les matières générales sont déjà là ; la technologie,
  // l'atelier et le stage s'ajoutent, chacun avec son coefficient.
  await allerA("nav.pedagogie");
  const cap1 = `CAP 1re année · ${texte("niveaux.cycle.technique")}`;
  await page.locator("#programme-niveau").selectOption({ label: cap1 });
  await expect(page.locator("#programme-serie")).toHaveValue(/.+/);
  await expect(page.locator("#programme-FR")).toBeChecked();
  await expect(page.locator("#programme-TPA")).not.toBeChecked();
  for (const code of ["TECH", "TPA", "STAGE"]) await page.locator(`#programme-${code}`).check();
  for (const [code, coefficient] of [["FR", "2"], ["ANG", "1"], ["MATH", "3"], ["PC", "2"], ["TECH", "4"], ["TPA", "5"], ["STAGE", "3"]] as const) {
    await page.locator(`#programme-${code}-coefficient`).fill(coefficient);
  }
  await page.getByRole("button", { name: texte("commun.enregistrer") }).click();
  await expect(page.getByRole("status").filter({ hasText: /Enregistré à/ })).toBeVisible();
  await capturer("91-programme-cap-electricite");

  // Deux semestres au CAP ; une école qui compte en trimestres les choisit année par année.
  await page.getByRole("button", { name: texte("pedagogie.onglet.periodes") }).click();
  await page.locator("#periodes-niveau").selectOption({ label: cap1 });
  await expect(page.getByText("1er semestre")).toBeVisible();
  await expect(page.getByText("2e semestre")).toBeVisible();
  await capturer("92-semestres-du-cap");
  await page.locator("#periodes-niveau").selectOption({ label: `BT 1re année · ${texte("niveaux.cycle.technique")}` });
  await page.getByRole("button", { name: texte("periodes.trois_trimestres") }).click();
  await expect(page.getByText("3e trimestre")).toBeVisible();
  await page.getByRole("button", { name: texte("periodes.deux_semestres") }).click();
  await expect(page.getByText("2e semestre")).toBeVisible();
  await expect(page.getByText("3e trimestre")).toHaveCount(0);
});

test("phase 6 : la classe CAP 1 Électricité inscrit ses apprentis, compose, puis publie les bulletins du 1er semestre", async () => {
  // La classe de la filière, puis ses frais : une nouvelle version de la grille couvre le CAP.
  await allerA("nav.classes");
  await page.locator("#classe-niveau").selectOption({ label: "CAP 1re année" });
  await page.locator("#classe-serie").selectOption({ label: "ELEC — Électricité" });
  await page.locator("#classe-libelle").fill("CAP 1 Électricité");
  await page.getByRole("button", { name: texte("classes.creer") }).click();
  await expect(page.getByRole("cell", { name: "CAP 1 Électricité", exact: true })).toBeVisible();
  await allerA("nav.frais");
  await page.getByRole("button", { name: texte("frais.nouvelle_version") }).click();
  const montants = page.locator('[id^="frais-ligne-"][id$="-montant"]');
  await expect(montants.first()).toBeVisible();
  const lignes = await montants.count();
  for (let i = 0; i < lignes; i++) {
    if ((await page.locator(`#frais-ligne-${i}-montant`).inputValue()) === "10000") await page.locator(`#frais-ligne-${i}-niveau-CAP1`).check();
  }
  await page.getByRole("button", { name: texte("frais.ajouter_ligne") }).click();
  await page.locator(`#frais-ligne-${lignes}-niveau-CAP1`).check();
  await page.locator(`#frais-ligne-${lignes}-montant`).fill("90000");
  await page.getByRole("button", { name: texte("frais.mettre_en_vigueur") }).click();
  await expect(page.getByRole("heading", { name: new RegExp(`version \\d+\\) · ${texte("frais.etat.en_vigueur")}`) })).toBeVisible();

  // Deux apprentis, sans matricule national : il est facultatif au CAP.
  for (const [matricule, nom, prenoms, mere] of [
    ["CAP/2026/001", "Diallo", "Moussa", "76 30 40 50"],
    ["CAP/2026/002", "Konaté", "Sékou", "66 30 40 51"],
  ] as const) {
    await allerA("nav.eleves");
    await page.getByRole("button", { name: texte("eleves.nouveau") }).click();
    await page.locator("#nouvel-eleve-matricule").fill(matricule);
    await page.locator("#nouvel-eleve-nom").fill(nom);
    await page.locator("#nouvel-eleve-prenoms").fill(prenoms);
    await page.locator("#nouvel-eleve-sexe").selectOption("masculin");
    await page.locator("#nouvel-eleve-naissance").fill("2009-05-14");
    await page.locator("#nouvel-eleve-lieu_naissance").fill("Ségou");
    await page.getByRole("button", { name: texte("eleves.creer") }).click();
    await expect(page.getByRole("heading", { name: `${prenoms} ${nom}` })).toBeVisible();
    await page.locator("#ajout-lien").selectOption("mere");
    await page.locator("#nouveau-responsable-nom").fill(nom);
    await page.locator("#nouveau-responsable-prenoms").fill("Kadiatou");
    await page.locator("#nouveau-responsable-telephone-0").fill(mere);
    await page.getByRole("button", { name: texte("responsables.creer_et_lier") }).click();
    await expect(page.locator("aside").getByText(`+223 ${mere}`)).toBeVisible();
    await page.locator("#inscription-classe").selectOption({ label: "CAP 1 Électricité · CAP 1re année" });
    await page.getByRole("button", { name: texte("inscriptions.inscrire_et_valider") }).click();
    await expect(page.getByText(`2026-2027 · CAP 1 Électricité · ${texte("inscriptions.statut.validee")}`)).toBeVisible();
  }
  // La liste réduite à l'apprenti, sa fiche ouverte.
  await page.locator("#eleves-recherche").fill("Konaté");
  await expect(page.getByRole("cell", { name: "CAP/2026/002" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Sékou Konaté" })).toBeVisible();
  await capturer("93-apprenti-inscrit-au-cap");

  // Les compositions du 1er semestre : l'atelier, puis les maths.
  await allerA("nav.notes");
  await page.locator("#notes-classe").selectOption({ label: "CAP 1 Électricité · CAP 1re année" });
  await expect(page.locator("#notes-periode")).toHaveValue(/.+/);
  await expect(page.locator("#notes-periode").locator("option:checked")).toHaveText("1er semestre");
  const composer = async (matiere: string, notes: readonly string[]) => {
    await page.locator("#notes-matiere").selectOption({ label: matiere });
    await page.locator("#evaluation-type").selectOption("composition");
    await page.getByRole("button", { name: texte("notes.creer") }).click();
    for (const [i, note] of notes.entries()) {
      await page.locator(`#note-${i + 1}`).fill(note);
      await page.locator(`#note-${i + 1}`).press("Enter");
    }
    await expect(page.getByRole("status").filter({ hasText: `${notes.length} élèves sur ${notes.length}` })).toBeVisible();
    await page.getByRole("button", { name: texte("notes.valider") }).click();
    await expect(page.getByText(texte("notes.validee"))).toBeVisible();
    await page.getByRole("button", { name: texte("notes.retour") }).click();
  };
  await composer("Travaux pratiques d'atelier", ["16", "12"]);
  await composer("Mathématiques", ["9", "15"]);

  // Les bulletins du 1er semestre, calculés, validés, publiés.
  await allerA("nav.bulletins");
  await page.locator("#bulletins-classe").selectOption({ label: "CAP 1 Électricité · CAP 1re année" });
  await expect(page.locator("#bulletins-periode").locator("option:checked")).toHaveText("1er semestre");
  await page.getByRole("button", { name: texte("bulletins.calculer") }).click();
  await expect(page.getByText("Bulletins calculés : 2.")).toBeVisible();
  await capturer("94-bulletins-du-premier-semestre");
  await page.getByRole("button", { name: texte("bulletins.valider") }).click();
  await expect(page.getByText("Bulletins validés : 2.")).toBeVisible();
  await page.getByRole("button", { name: texte("bulletins.publier") }).click();
  await expect(page.getByText(texte("bulletins.publies"))).toBeVisible();
  await expect(page.locator("iframe")).toBeVisible({ timeout: 30_000 });
  // Le visionneur PDF de WebView2 rend la page hors du DOM : aucun signal à attendre.
  await page.waitForTimeout(4000);
  await page.locator("iframe").scrollIntoViewIfNeeded();
  await capturer("95-bulletin-cap-publie");
});

test("phase 7 : la direction règle sa paie, active la retraite INPS, crée une prime de transport et l'attribue", async () => {
  await allerA("nav.paie");
  await expect(page.getByRole("heading", { name: texte("paie.titre") })).toBeVisible();
  await page.getByRole("tab", { name: texte("paie.onglet.elements") }).click();
  // Douze mois payés et le trentième : les réglages livrés.
  await expect(page.locator("#paie-douze-mois")).toBeChecked();
  await expect(page.locator("#paie-prorata")).toHaveValue("trentieme");

  // La retraite INPS, proposée au taux connu, s'active.
  await page.getByRole("cell", { name: "Retraite INPS", exact: true }).click();
  await expect(page.locator("#calcul-taux")).toHaveValue("3,6");
  await page.locator("#element-actif").check();
  await page.getByRole("button", { name: texte("paie.element.enregistrer") }).click();
  await expect(page.getByRole("row").filter({ hasText: "Retraite INPS" }).filter({ hasText: texte("paie.element.actif") }).first()).toBeVisible();
  await page.getByRole("button", { name: texte("commun.fermer") }).click();

  // L'impôt sur les salaires : son barème annuel proposé, inactif.
  await page.getByRole("cell", { name: "Impôt sur les traitements et salaires", exact: true }).click();
  await expect(page.locator("#calcul-tranche-7-taux")).toHaveValue("37");
  await expect(page.locator("#assiette-INPS_RET")).toBeChecked();
  await capturer("96-elements-de-paie");
  await page.getByRole("button", { name: texte("commun.fermer") }).click();

  // Une prime de transport de l'école, fixe, aux permanents.
  await page.locator("#element-libelle").fill("Prime de transport");
  await page.locator("#element-nature-vacation").uncheck();
  await page.locator("#element-nature-stage").uncheck();
  await page.locator("#calcul-montant").fill("10 000");
  await page.getByRole("button", { name: texte("paie.element.creer") }).click();
  await expect(page.getByRole("cell", { name: "Prime de transport", exact: true })).toBeVisible();

  // Attribuée à la secrétaire dès octobre.
  await page.locator("#attribution-membre").selectOption({ label: "Mariam Keïta · Secrétaire" });
  await page.locator("#attribution-element").selectOption({ label: "Prime de transport" });
  await page.locator("#attribution-montant").fill("10000");
  await page.getByRole("button", { name: texte("paie.attribution.ajouter") }).click();
  await expect(page.getByText(/Mariam Keïta · Prime de transport · 10.000.F · depuis octobre 2026/)).toBeVisible();
  await page.getByRole("heading", { name: texte("paie.attributions") }).scrollIntoViewIfNeeded();
  await capturer("97-prime-attribuee");

  // La situation de famille de la cuisinière, pour l'impôt.
  await allerA("nav.personnel");
  await page.getByRole("cell", { name: "Kadiatou Diarra" }).click();
  await page.locator("#membre-situation").selectOption("marie");
  await page.locator("#membre-enfants").fill("2");
  await page.getByRole("button", { name: texte("commun.enregistrer"), exact: true }).click();
  await expect(page.getByText(texte("personnel.enregistre"))).toBeVisible();
});

test("phase 10 : la fin d'année propose, la direction valide le jardin, la clôture attend les autres décisions", async () => {
  await allerA("nav.fin_annee");
  await expect(page.getByRole("heading", { name: "Fin de l'année 2026-2027" })).toBeVisible();
  // La moyenne de passage du 1er cycle, livrée à 5 sur 10, reprise du CAP.
  await expect(page.locator("#passage-premier_cycle")).toHaveValue("5");
  await expect(page.locator("#passage-lycee")).toHaveValue("10");
  await capturer("98-fin-annee");

  // Au jardin, l'enfant passe à la section suivante : la direction valide la proposition.
  await page.getByRole("cell", { name: "PS", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Décisions de la PS" })).toBeVisible();
  await page.getByRole("button", { name: /^Valider les propositions/ }).click();
  await expect(page.getByRole("button", { name: "Valider les propositions (0)" })).toBeDisabled();
  await expect(page.locator("table").last().locator("select").first()).toHaveValue("admis");
  await capturer("99-decisions-jardin");

  // Les autres classes n'ont pas encore leurs bulletins de fin d'année : la clôture attend.
  await expect(page.getByRole("button", { name: "Clôturer l'année 2026-2027" })).toBeDisabled();
  await expect(page.getByText(/élève\(s\) sans décision : la clôture attend/)).toBeVisible();
});

test("phase 7 : la paie d'octobre se calcule, une prime s'ajoute, la direction la valide", async () => {
  await allerA("nav.paie");
  await expect(page.getByRole("tab", { name: texte("paie.onglet.mois"), selected: true })).toBeVisible();
  // Le 14 octobre, c'est encore la paie de septembre qui se prépare : la direction prend octobre.
  await expect(page.locator("#paie-mois")).toHaveValue("2026-09");
  await page.locator("#paie-mois").fill("2026-10");
  await page.getByRole("button", { name: "Calculer la paie d'octobre 2026" }).click();
  // La secrétaire et la cuisinière au mois, avec la prime de transport des permanents ; le
  // fonctionnaire vacataire, ses 22 h 30 visées.
  await expect(page.getByRole("cell", { name: "Kadiatou Diarra" })).toBeVisible();
  await expect(page.getByRole("row").filter({ hasText: "Bakary Sangaré" }).getByRole("cell", { name: /^56.250.F$/ }).first()).toBeVisible();

  // Une prime du mois pour la cuisinière, avec son motif.
  await page.locator("#saisie-membre").selectOption({ label: "Kadiatou Diarra" });
  await page.locator("#saisie-element").selectOption({ label: "Prime" });
  await page.locator("#saisie-montant").fill("5 000");
  await page.locator("#saisie-motif").fill("Repas de la fête de l'école");
  await page.getByRole("button", { name: texte("paie.mois.ajouter"), exact: true }).click();
  await expect(page.getByText(/Kadiatou Diarra · Prime · Repas de la fête de l'école · 5.000.F/)).toBeVisible();
  await page.getByRole("cell", { name: "Kadiatou Diarra" }).click();
  await expect(page.getByRole("heading", { name: "Bulletin de Kadiatou Diarra" })).toBeVisible();
  await capturer("100-paie-du-mois");

  // Validée : chaque bulletin reçoit son numéro, la paie ne change plus.
  await page.getByRole("button", { name: "Valider la paie d'octobre 2026" }).click();
  await page.getByRole("button", { name: /^Confirmer : 3 bulletin\(s\)/ }).click();
  await expect(page.getByText(/^Validée le/)).toBeVisible();
  await expect(page.getByRole("cell", { name: /^PAI-2026-00000[123]$/ })).toHaveCount(3);
  await expect(page.getByRole("button", { name: texte("paie.mois.ajouter"), exact: true })).toHaveCount(0);
  await capturer("101-paie-validee");

  // L'état de paie à faire signer pour acquit, en A4 paysage.
  await page.getByRole("button", { name: texte("paie.document.etat_paie") }).click();
  // Le visionneur PDF de WebView2 rend la page hors du DOM : aucun signal à attendre.
  await page.waitForTimeout(3000);
  await page.locator("iframe").last().scrollIntoViewIfNeeded();
  await capturer("103-etat-de-paie");
});

test("phase 7 : un salaire se paie en deux fois, le reste du mois d'un geste, une avance se verse", async () => {
  await allerA("nav.paie");
  await page.getByRole("tab", { name: texte("paie.onglet.paiements") }).click();
  await expect(page.getByRole("heading", { name: /^Salaires à payer · 210.490.F$/ })).toBeVisible();

  // La cuisinière reçoit 30 000 F par Orange Money, avec la référence de la transaction.
  await page.getByRole("row").filter({ hasText: "Kadiatou Diarra" }).getByRole("cell", { name: "Kadiatou Diarra" }).click();
  await page.locator("#paiement-montant").fill("30 000");
  await page.locator("#paiement-mode").selectOption("orange_money");
  await page.locator("#paiement-reference").fill("OM-4521");
  await page.getByRole("button", { name: texte("paiement.payer"), exact: true }).click();
  await expect(page.getByRole("heading", { name: /^Salaires à payer · 180.490.F$/ })).toBeVisible();

  // Le reste d'octobre, d'un geste, chacun selon sa fiche.
  await page.getByRole("button", { name: "Payer le reste d'octobre 2026" }).click();
  await page.getByRole("button", { name: /^Confirmer : 3 personne\(s\)/ }).click();
  await expect(page.getByText(texte("paiement.rien_du"))).toBeVisible();

  // Une avance de 30 000 F à la secrétaire, remboursée en trois mois.
  await page.locator("#avance-membre").selectOption({ label: "Mariam Keïta" });
  await page.locator("#avance-montant").fill("30 000");
  await page.locator("#avance-mensualites").fill("3");
  await page.locator("#avance-premier").fill("2026-11");
  await page.locator("#avance-motif").fill("Rentrée des enfants");
  await page.locator("#avance-mode").selectOption("orange_money");
  await page.locator("#avance-reference").fill("OM-9001");
  await page.getByRole("button", { name: texte("avance.accorder") }).click();
  await expect(page.getByText(/Mariam Keïta · 30.000.F · 10.000.F par mois · reste 30.000.F/)).toBeVisible();
  await capturer("102-paiements-et-avances");
});

test("phase 8 : l'école décrit ses comptes, la BDM et son Orange Money, et ses catégories", async () => {
  await allerA("nav.tresorerie");
  await expect(page.getByRole("heading", { name: texte("tresorerie.titre") })).toBeVisible();
  // La caisse est livrée ; l'école ajoute son compte en banque, avec son solde au 1er octobre.
  await expect(page.getByRole("cell", { name: "Caisse", exact: true }).first()).toBeVisible();
  await page.locator("#compte-libelle").fill("Compte courant");
  await page.locator("#compte-organisme").fill("BDM");
  await page.locator("#compte-numero").fill("ML016 01001 012345678901 23");
  await page.locator("#compte-solde").fill("250 000");
  await page.locator("#compte-ouvert").fill("2026-10-01");
  await page.getByRole("button", { name: texte("tresorerie.ajouter_compte") }).click();
  await expect(page.getByRole("cell", { name: "Compte courant" })).toBeVisible();
  // Puis son compte Orange Money.
  await page.locator("#compte-nature").selectOption("mobile_money");
  await page.locator("#compte-libelle").fill("Orange Money de l'école");
  await page.locator("#compte-organisme").fill("Orange Money");
  await page.locator("#compte-numero").fill("76 00 00 01");
  await expect(page.locator("#compte-solde")).toHaveValue("0");
  await page.locator("#compte-ouvert").fill("2026-10-01");
  await page.getByRole("button", { name: texte("tresorerie.ajouter_compte") }).click();
  await expect(page.getByRole("cell", { name: "Orange Money de l'école" })).toBeVisible();

  // Une catégorie de dépense à elle : la communication (affiches, crédit téléphonique).
  await page.locator("#categorie-depense").fill("Communication");
  await page.locator("form").filter({ has: page.locator("#categorie-depense") }).getByRole("button", { name: texte("tresorerie.ajouter") }).click();
  await expect(page.locator("input[value='Communication']")).toBeVisible();
  await capturer("104-tresorerie-comptes");
});

test("phase 8 : des craies payées d'un geste, une toiture qui attend l'accord puis se paie par virement", async () => {
  await allerA("nav.depenses");
  await expect(page.getByRole("heading", { name: texte("depense.titre"), exact: true })).toBeVisible();
  // Sous le seuil de 50 000 F : saisie et payée en espèces de la caisse.
  await page.locator("#depense-categorie").selectOption({ label: "Fournitures" });
  await page.locator("#depense-libelle").fill("Craies et feutres");
  await page.locator("#depense-beneficiaire").fill("Librairie Kanté");
  await page.locator("#depense-montant").fill("7 500");
  await page.locator("#depense-compte").selectOption({ label: "Caisse" });
  await page.getByRole("button", { name: texte("depense.enregistrer") }).click();
  await expect(page.getByRole("row").filter({ hasText: "Craies et feutres" }).getByText(texte("depense.statut.payee"))).toBeVisible();

  // Au-dessus : la réparation de la toiture attend l'accord de la direction.
  await page.locator("#depense-categorie").selectOption({ label: "Entretien et réparations" });
  await page.locator("#depense-libelle").fill("Réparation de la toiture");
  await page.locator("#depense-beneficiaire").fill("Entreprise Diakité");
  await page.locator("#depense-montant").fill("150 000");
  await expect(page.getByText(/attendra l'accord de la direction/)).toBeVisible();
  await page.getByRole("button", { name: texte("depense.enregistrer") }).click();
  await expect(page.getByRole("heading", { name: texte("depense.en_attente") })).toBeVisible();
  await page.getByRole("button", { name: texte("depense.accorder") }).click();
  // Accordée, elle se paie par virement depuis le compte de la BDM.
  const payer = page.locator("form").filter({ hasText: "Réparation de la toiture" });
  await payer.locator("select").first().selectOption({ label: "Compte courant" });
  await payer.getByRole("textbox", { name: texte("encaissement.reference") }).fill("VIR-118");
  await payer.getByRole("button", { name: texte("depense.payer") }).click();
  await expect(page.getByRole("row").filter({ hasText: "Réparation de la toiture" }).getByText(texte("depense.statut.payee"))).toBeVisible();
  await capturer("105-depenses");
});

test("phase 8 : la facture de la quincaillerie se paie en deux fois, le reste dû se lit", async () => {
  await allerA("nav.fournisseurs");
  await expect(page.getByRole("heading", { name: texte("fournisseur.titre"), exact: true })).toBeVisible();
  await page.locator("#fournisseur-nom").fill("Quincaillerie Coulibaly");
  await page.locator("#fournisseur-telephone").fill("76 12 34 56");
  await page.locator("#fournisseur-fourniture").fill("Peinture et matériaux");
  await page.getByRole("button", { name: texte("fournisseur.enregistrer") }).click();
  await page.getByRole("cell", { name: "Quincaillerie Coulibaly" }).click();

  // La facture de la peinture des classes, à payer avant le 10 novembre.
  await page.locator("#facture-numero").fill("F-2026-041");
  await page.locator("#facture-date").fill("2026-10-12");
  await page.locator("#facture-echeance").fill("2026-11-10");
  await page.locator("#facture-libelle").fill("Peinture des classes du 1er cycle");
  await page.locator("#facture-categorie").selectOption({ label: "Entretien et réparations" });
  await page.locator("#facture-montant").fill("120 000");
  await page.getByRole("button", { name: texte("fournisseur.facture.enregistrer") }).click();
  await page.getByRole("cell", { name: "F-2026-041" }).click();

  // Un acompte de 40 000 F en espèces, sous le seuil : payé d'un geste.
  await expect(page.locator("#facture-paiement-montant")).toHaveValue("120000");
  await page.locator("#facture-paiement-montant").fill("40 000");
  await page.locator("#facture-compte").selectOption({ label: "Caisse" });
  await page.getByRole("button", { name: texte("fournisseur.facture.payer") }).click();
  await expect(page.getByText(/Payé 40.000.F · en attente 0.F · reste dû 80.000.F/)).toBeVisible();
  await expect(page.getByRole("row").filter({ hasText: "Quincaillerie Coulibaly" }).getByRole("cell", { name: /80.000.F/ })).toBeVisible();
  await capturer("106-fournisseurs");
});

test("phase 8 : la recette se dépose à la BDM, le livre du compte se coche contre le relevé", async () => {
  await allerA("nav.livres");
  await expect(page.getByRole("heading", { name: texte("livre.titre"), exact: true })).toBeVisible();
  // Le dépôt de la recette : il sort de la caisse et entre au compte courant.
  await page.locator("#transfert-source").selectOption({ label: "Caisse" });
  await page.locator("#transfert-destination").selectOption({ label: "Compte courant" });
  await page.locator("#transfert-montant").fill("50 000");
  await page.locator("#transfert-reference").fill("BV-0042");
  await page.locator("#transfert-motif").fill("Dépôt de la recette du jour");
  await page.getByRole("button", { name: texte("livre.transferer") }).click();
  await expect(page.locator("#transfert-montant")).toHaveValue("");
  await expect(page.getByRole("row").filter({ hasText: "Compte courant" }).getByRole("cell", { name: /^150.000.F$/ })).toBeVisible();

  // Le livre d'octobre du compte courant : le dépôt, coché contre le relevé de la BDM.
  await page.getByRole("cell", { name: "Compte courant", exact: true }).click();
  const depot = page.getByRole("row").filter({ hasText: "Dépôt de la recette du jour" });
  await expect(depot).toBeVisible();
  await depot.click();
  await expect(depot.getByText(texte("livre.pointee"))).toBeVisible();
  await capturer("107-comptes-et-soldes");
});

test("phase 8 : le journal d'octobre range les recettes et les dépenses, le budget avertit d'un dépassement", async () => {
  await allerA("nav.journal_tresorerie");
  await expect(page.getByRole("heading", { name: texte("journal_tresorerie.titre"), exact: true })).toBeVisible();
  await expect(page.getByRole("cell", { name: "Entretien et réparations" }).first()).toBeVisible();
  // 100 000 F prévus pour l'entretien de l'année : la toiture et la peinture les dépassent déjà.
  await page.locator("#budget-categorie").selectOption({ label: "Dépense · Entretien et réparations" });
  await page.locator("#budget-montant").fill("100 000");
  await page.locator("form").filter({ has: page.locator("#budget-montant") }).getByRole("button", { name: texte("commun.enregistrer") }).click();
  await expect(page.getByText(/catégorie\(s\) de dépense ont dépassé leur prévu/)).toBeVisible();
  await capturer("108-journal-et-budget");
});

test("phase 8 : une séance se place dans la journée de sa classe, l'emploi du temps s'imprime", async () => {
  await allerA("nav.emploi_du_temps");
  await expect(page.getByRole("heading", { name: texte("emploi.titre"), exact: true })).toBeVisible();
  // Le lundi de 8 h à 9 h, la première matière du programme, avec son enseignant.
  await page.locator("#seance-jour").selectOption({ label: "Lundi" });
  await page.locator("#seance-debut").fill("08:00");
  await page.locator("#seance-fin").fill("09:00");
  await page.getByRole("button", { name: texte("emploi.placer") }).click();
  await expect(page.getByRole("button", { name: /08:00 – 09:00/ })).toBeVisible();
  // Le mercredi après-midi, il n'y a pas classe : la séance est refusée.
  await page.locator("#seance-jour").selectOption({ label: "Mercredi" });
  await page.locator("#seance-debut").fill("15:00");
  await page.locator("#seance-fin").fill("16:00");
  await page.getByRole("button", { name: texte("emploi.placer") }).click();
  await expect(page.getByText(texte("seance.hors_journee"))).toBeVisible();
  await page.getByRole("button", { name: /Imprimer l'emploi du temps de la/ }).click();
  await expect(page.locator("iframe")).toBeVisible();
  // Le visionneur PDF de WebView2 rend la page hors du DOM : aucun signal à attendre.
  await page.waitForTimeout(4000);
  await page.locator("iframe").scrollIntoViewIfNeeded();
  await capturer("109-emploi-du-temps");
});

test("phase 8 : l'école active sa cantine et compte les repas, la liste du jour s'imprime", async () => {
  await allerA("nav.tresorerie");
  await page.locator("#reglage-cantine").check();
  await page.locator("#reglage-repas").check();
  await page.locator("form").filter({ has: page.locator("#reglage-cantine") }).getByRole("button", { name: texte("commun.enregistrer") }).click();
  await expect(page.locator("#reglage-repas")).toBeChecked();
  await allerA("nav.cantine");
  await expect(page.getByRole("heading", { name: texte("cantine.titre"), exact: true })).toBeVisible();
  await expect(page.getByText(/repas pointés/)).toBeVisible();
  await page.getByRole("button", { name: texte("cantine.imprimer") }).click();
  await expect(page.locator("iframe")).toBeVisible();
  await capturer("110-cantine");
});

test("phase 8 : le tableau de bord montre les soldes, les dépenses du mois et le budget dépassé", async () => {
  await allerA("nav.accueil");
  await expect(page.getByRole("heading", { name: texte("tableau.tresorerie"), exact: true })).toBeVisible();
  await expect(page.getByText(texte("tableau.depenses_du_mois"))).toBeVisible();
  await expect(page.getByText(/catégorie\(s\) dépassée\(s\)/)).toBeVisible();
  await capturer("111-tableau-de-bord-tresorerie");
});

test("version 1.1 : un don en espèces et une location par virement entrent dans leurs comptes", async () => {
  await allerA("nav.recettes");
  await expect(page.getByRole("heading", { name: texte("recette.titre"), exact: true })).toBeVisible();
  // Le don de l'association des parents, en espèces dans la caisse ouverte.
  await page.locator("#recette-libelle").fill("Don pour la bibliothèque");
  await page.locator("#recette-payeur").fill("Association des parents d'élèves");
  await page.locator("#recette-montant").fill("100 000");
  await page.locator("#recette-compte").selectOption({ label: "Caisse" });
  await page.getByRole("button", { name: texte("recette.enregistrer") }).click();
  await expect(page.getByRole("row").filter({ hasText: "Don pour la bibliothèque" })).toBeVisible();
  // La location de la salle par la mairie, par virement à la BDM.
  await page.locator("#recette-libelle").fill("Location de la salle pour une réunion");
  await page.locator("#recette-payeur").fill("Mairie de la commune IV");
  await page.locator("#recette-montant").fill("25 000");
  await page.locator("#recette-compte").selectOption({ label: "Compte courant" });
  await page.getByRole("textbox", { name: texte("encaissement.reference") }).fill("VIR-2026-0412");
  await page.getByRole("button", { name: texte("recette.enregistrer") }).click();
  await expect(page.getByRole("row").filter({ hasText: "Location de la salle pour une réunion" })).toBeVisible();
  await expect(page.getByText(/125.000.F reçus/).first()).toBeVisible();
  await capturer("112-autres-recettes");
});

test("version 1.1 : la photo d'une élève se prend à la webcam du poste", async () => {
  await allerA("nav.eleves");
  await page.getByRole("button", { name: texte("eleves.nouveau") }).click();
  await page.locator("#nouvel-eleve-matricule").fill("L/2026/120");
  await page.locator("#nouvel-eleve-nom").fill("Diakité");
  await page.locator("#nouvel-eleve-prenoms").fill("Kadiatou");
  await page.locator("#nouvel-eleve-sexe").selectOption("feminin");
  await page.locator("#nouvel-eleve-naissance").fill("2015-03-02");
  await page.locator("#nouvel-eleve-lieu_naissance").fill("Bamako");
  await page.getByRole("button", { name: texte("eleves.creer") }).click();
  await expect(page.getByRole("heading", { name: "Kadiatou Diakité" })).toBeVisible();
  await page.getByRole("button", { name: texte("eleves.webcam") }).click();
  // La caméra s'allume ; la photo prise, elle s'éteint et la photo paraît sur la fiche.
  await expect(page.locator("video")).toBeVisible();
  await expect.poll(() => page.locator("video").evaluate((v) => (v as HTMLVideoElement).videoWidth), { timeout: 15_000 }).toBeGreaterThan(0);
  await page.getByRole("button", { name: texte("eleves.prendre_photo") }).click();
  await expect(page.getByRole("img", { name: texte("eleves.photo") })).toBeVisible();
  await expect(page.locator("video")).toHaveCount(0);
  await capturer("113-photo-webcam");
});

test("version 1.1 : le conseil de classe donne un blâme, qui va au bulletin du trimestre", async () => {
  await allerA("nav.discipline");
  await expect(page.getByRole("heading", { name: texte("discipline.titre"), exact: true })).toBeVisible();
  await page.locator("#sanction-classe").selectOption({ label: "7e A" });
  await page.locator("#sanction-eleve").selectOption({ index: 1 });
  await page.locator("#sanction-nature").selectOption("blame");
  await page.locator("#sanction-motif").fill("Bavardages répétés en classe");
  await page.locator("#sanction-decidee").fill("Conseil de classe");
  await page.locator("#sanction-periode").selectOption({ index: 1 });
  await page.getByRole("button", { name: texte("discipline.noter") }).click();
  const ligne = page.getByRole("row").filter({ hasText: "Bavardages répétés en classe" });
  await expect(ligne).toBeVisible();
  await expect(ligne.getByText(/au bulletin du/)).toBeVisible();
  await capturer("114-discipline");
});

test("version 1.1 : le journal d'audit se vérifie, ses archives aussi", async () => {
  await allerA("nav.journal");
  await expect(page.getByRole("heading", { name: texte("journal.archives") })).toBeVisible();
  await expect(page.getByRole("button", { name: texte("journal.suivante") })).toBeEnabled();
  await expect(page.getByRole("button", { name: texte("journal.verifier_archive") })).toBeVisible();
  await capturer("115-archives-du-journal");
});

test("version 1.1 : l'école relève les absences à chaque séance ; absente au seul cours du matin, l'élève compte la demi-journée", async () => {
  await allerA("nav.parametres");
  const reglage = page.locator('[id="param-absence.par_seance"]');
  await reglage.check();
  await reglage.locator("xpath=ancestor::div[contains(@class,'py-2')][1]").getByRole("button", { name: texte("commun.enregistrer") }).click();
  // Le mercredi de 8 h à 9 h en 7e B.
  await allerA("nav.emploi_du_temps");
  await page.locator("#emploi-classe").selectOption({ label: "7e B" });
  await page.locator("#seance-jour").selectOption({ label: "Mercredi" });
  await page.locator("#seance-debut").fill("08:00");
  await page.locator("#seance-fin").fill("09:00");
  await page.getByRole("button", { name: texte("emploi.placer") }).click();
  await expect(page.getByRole("button", { name: /08:00 – 09:00/ }).last()).toBeVisible();
  // Le mercredi 14 octobre, un élève absent à cette séance : la matinée lui est comptée.
  await allerA("nav.absences");
  await page.locator("#absences-classe").selectOption({ label: "7e B · 7ème année" });
  await page.locator("#absences-date").fill("2026-10-14");
  await expect(page.getByRole("heading", { name: texte("absences.par_seance") })).toBeVisible();
  const caseSeance = page.getByRole("checkbox", { name: /absent à la séance de 08:00/, disabled: false }).first();
  const eleve = ((await caseSeance.getAttribute("aria-label")) ?? "").replace(/ absent à la séance .*$/, "");
  await caseSeance.check();
  await page.getByRole("button", { name: texte("absences.enregistrer_par_seance") }).click();
  await expect(caseSeance).toBeChecked();
  // Le relevé du jour en déduit sa matinée d'absence.
  await expect(page.getByRole("textbox", { name: `Matin de ${eleve}` })).toHaveValue("a");
  await capturer("116-absences-par-seance");
});

test("version 1.1 : le relevé de la BDM s'importe ; le virement de la toiture et la location se cochent, les frais restent à part", async () => {
  const releve = join(dossier, "releve-bdm-octobre.csv");
  const lignes = [
    "Date;Libellé;Débit;Crédit;Référence",
    "14/10/2026;VIREMENT ENTREPRISE DIAKITE;150 000;;VIR 118",
    "15/10/2026;VIR MAIRIE COMMUNE IV;;25 000;",
    "16/10/2026;FRAIS DE TENUE DE COMPTE;2 500;;FR-10",
  ];
  writeFileSync(releve, `${String.fromCharCode(0xfeff)}${lignes.join("\r\n")}\r\n`, "utf8");
  await allerA("nav.livres");
  await page.getByRole("cell", { name: "Compte courant", exact: true }).click();
  await expect(page.getByRole("row").filter({ hasText: "Réparation de la toiture" })).toBeVisible();
  // Le dialogue d'ouverture du système ne se pilote pas : son appel reçoit le chemin du relevé.
  await page.evaluate((chemin) => {
    const fenetre = window as unknown as { fetchOriginal?: typeof fetch };
    const original = window.fetch.bind(window);
    fenetre.fetchOriginal = original;
    window.fetch = (entree, options) => {
      const url = decodeURIComponent(typeof entree === "string" ? entree : entree instanceof URL ? entree.href : entree.url);
      if (url.includes("ipc.localhost/plugin:dialog|open")) {
        return Promise.resolve(new Response(JSON.stringify(chemin), { status: 200, headers: { "Tauri-Response": "ok", "content-type": "application/json" } }));
      }
      return original(entree, options);
    };
  }, releve);
  await page.getByRole("button", { name: texte("livre.choisir_releve") }).click();
  // Les colonnes se devinent d'après l'en-tête.
  await expect(page.locator("#releve-debit")).toHaveValue("2");
  await expect(page.locator("#releve-credit")).toHaveValue("3");
  await expect(page.locator("#releve-reference")).toHaveValue("4");
  await page.evaluate(() => {
    const fenetre = window as unknown as { fetchOriginal?: typeof fetch };
    if (fenetre.fetchOriginal !== undefined) window.fetch = fenetre.fetchOriginal;
  });
  await page.getByRole("button", { name: texte("livre.rapprocher") }).click();
  await expect(page.getByText(/^2 ligne\(s\) cochée\(s\) sur 3 lue\(s\)/)).toBeVisible();
  await expect(page.getByText(/Pas dans le livre : 16 octobre 2026 · .2.500.F · FR-10/)).toBeVisible();
  await expect(page.getByRole("row").filter({ hasText: "Réparation de la toiture" }).getByText(texte("livre.pointee"))).toBeVisible();
  await expect(page.getByRole("row").filter({ hasText: "Location de la salle pour une réunion" }).getByText(texte("livre.pointee"))).toBeVisible();
  await capturer("117-import-du-releve");
});

test("version 1.1 : une liste du personnel s'importe après vérification ; la personne déjà au registre est laissée de côté", async () => {
  const liste = join(dossier, "personnel-a-importer.csv");
  const lignes = [
    "Nom;Prénoms;Catégorie;Fonction;Sexe;Date de naissance;Téléphone;Date d'arrivée;Matricule;Fonctionnaire",
    "Dembélé;Mariam;Enseignement;Professeure de français;F;14/02/1988;76 21 43 65;01/10/2026;;non",
    "Konaté;Seydou;Appui;Gardien;M;;66 10 20 30;01/10/2026;;non",
    "Sangaré;Bakary;Enseignement;Professeur de mathématiques;M;;;;;",
    "Touré;Aminata;Cantine;Cuisinière;F;;;;;",
  ];
  writeFileSync(liste, `${String.fromCharCode(0xfeff)}${lignes.join("\r\n")}\r\n`, "utf8");
  await allerA("nav.personnel");
  await page.getByRole("button", { name: texte("import_personnel.ouvrir") }).click();
  await expect(page.getByRole("heading", { name: texte("import_personnel.titre") })).toBeVisible();
  await page.evaluate((chemin) => {
    const fenetre = window as unknown as { fetchOriginal?: typeof fetch };
    const original = window.fetch.bind(window);
    fenetre.fetchOriginal = original;
    window.fetch = (entree, options) => {
      const url = decodeURIComponent(typeof entree === "string" ? entree : entree instanceof URL ? entree.href : entree.url);
      if (url.includes("ipc.localhost/plugin:dialog|open")) {
        return Promise.resolve(new Response(JSON.stringify(chemin), { status: 200, headers: { "Tauri-Response": "ok", "content-type": "application/json" } }));
      }
      return original(entree, options);
    };
  }, liste);
  await page.getByRole("button", { name: texte("import_personnel.choisir") }).click();
  await expect(page.getByText("2 personne(s) prête(s) à importer sur 4 ligne(s) ; les autres sont laissées de côté.")).toBeVisible();
  await page.evaluate(() => {
    const fenetre = window as unknown as { fetchOriginal?: typeof fetch };
    if (fenetre.fetchOriginal !== undefined) window.fetch = fenetre.fetchOriginal;
  });
  await expect(page.getByRole("row").filter({ hasText: "Sangaré" }).getByText(texte("import_personnel.deja_au_registre"))).toBeVisible();
  await expect(page.getByRole("row").filter({ hasText: "Touré" }).getByText(texte("import_personnel.categorie_inconnue"))).toBeVisible();
  await capturer("118-import-du-personnel");
  await page.getByRole("button", { name: "Importer 2 personne(s)" }).click();
  // L'import commence par une sauvegarde : il lui faut plus que les cinq secondes ordinaires.
  await expect(page.getByText("2 fiche(s) créée(s) ; 2 ligne(s) laissée(s) de côté.")).toBeVisible({ timeout: 30_000 });
  await page.getByRole("button", { name: texte("import.fermer") }).click();
  await expect(page.getByRole("cell", { name: "Mariam Dembélé" })).toBeVisible();
  await expect(page.getByRole("cell", { name: "Seydou Konaté" })).toBeVisible();
});

test("phase 5 : la mère d'Awa ne veut plus de relances, elle garde l'avis de bulletin ; les réglages des SMS", async () => {
  // Les messages aux familles sont un module : l'école l'allume dans les paramètres.
  await allerA("nav.parametres");
  await expect(page.getByRole("heading", { name: texte("parametres.theme.messages") })).toHaveCount(0);
  await page.locator("#module-sms").check();
  await expect(page.getByRole("heading", { name: texte("parametres.theme.messages") })).toBeVisible();
  await page.getByRole("heading", { name: texte("modules.titre") }).scrollIntoViewIfNeeded();
  await capturer("128-modules-ecole");
  await page.keyboard.press("Control+k");
  const recherche = page.getByRole("dialog", { name: texte("recherche.titre") });
  await recherche.getByRole("searchbox").fill("Awa Traoré");
  await expect(recherche.getByText("Awa Traoré").first()).toBeVisible();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("heading", { name: "Awa Traoré" })).toBeVisible();
  const mere = page.locator("aside li").filter({ hasText: "Mariam Coulibaly" });
  const relances = mere.getByLabel(texte("messages.type.impaye"));
  await expect(relances).toBeChecked();
  // La case suit le clic dès que l'interface l'a pris en compte : on attend son nouvel état.
  await relances.click();
  await expect(relances).not.toBeChecked();
  await expect(mere.getByLabel(texte("messages.type.bulletin"))).toBeChecked();
  await capturer("119-messages-du-responsable");

  await allerA("nav.parametres");
  const section = page.getByRole("heading", { name: texte("parametres.theme.messages") });
  await expect(section).toBeVisible();
  await expect(page.locator('[id="param-sms.fenetre_envoi"]')).toHaveValue("7-20");
  await section.scrollIntoViewIfNeeded();
  await capturer("120-reglages-des-sms");
});

test("phase 5 : l'école essaie l'envoi sans clé ; le SMS d'essai part à son tour et revient remis", async () => {
  await allerA("nav.parametres");
  const enregistrer = async (id: string) => {
    await page.locator(`[id="${id}"]`).locator("xpath=ancestor::div[1]").getByRole("button", { name: texte("commun.enregistrer") }).click();
  };
  await page.locator('[id="param-sms.canal"]').selectOption("simule");
  await enregistrer("param-sms.canal");
  // Les heures d'envoi couvrent la journée : le scénario peut tourner à toute heure.
  await page.locator('[id="param-sms.fenetre_envoi"]').fill("0-24");
  await enregistrer("param-sms.fenetre_envoi");
  await expect(page.locator('[id="param-sms.fenetre_envoi"]')).toHaveValue("0-24");

  await allerA("nav.messages");
  await expect(page.getByText(texte("messages.canal_simule"))).toBeVisible();
  await page.locator("#message-essai-numero").fill("76 00 00 99");
  await page.getByRole("button", { name: texte("messages.essai_envoyer") }).click();
  // La file part toutes les cinq secondes ; l'accusé de remise revient aussitôt.
  const ligne = page.getByRole("row").filter({ hasText: "+223 76 00 00 99" });
  await expect(ligne.getByText(texte("messages.etat_message.remis"))).toBeVisible({ timeout: 20_000 });
  await ligne.click();
  await expect(page.getByRole("heading", { name: `${texte("messages.nature.essai")} vers +223 76 00 00 99` })).toBeVisible();
  await capturer("121-file-des-sms");
});

test("phase 5 : la clé modem s'essaie, garde son port, envoie et lit le crédit de sa puce", async () => {
  await allerA("nav.parametres");
  const enregistrer = async (id: string) => {
    await page.locator(`[id="${id}"]`).locator("xpath=ancestor::div[1]").getByRole("button", { name: texte("commun.enregistrer") }).click();
  };
  await page.locator('[id="param-sms.canal"]').selectOption("modem");
  await enregistrer("param-sms.canal");
  await page.locator('[id="param-sms.ussd_solde"]').fill("#111#");
  await enregistrer("param-sms.ussd_solde");
  await expect(page.locator('[id="param-sms.ussd_solde"]')).toHaveValue("#111#");

  await allerA("nav.messages");
  await expect(page.getByText(texte("messages.canal_modem_sans_port"))).toBeVisible();
  await page.locator("#cle-port").selectOption("simule");
  await page.getByRole("button", { name: texte("messages.essayer_cle") }).click();
  await expect(page.getByText("Clé trouvée : SIMULE Cle GSM simulee")).toBeVisible();
  await expect(page.getByText("Crédit : Votre solde est de 5 000 FCFA.")).toBeVisible();
  await page.getByRole("button", { name: texte("messages.choisir_port") }).click();
  await expect(page.getByText(texte("messages.canal_modem_simulee"))).toBeVisible();

  await page.locator("#message-essai-numero").fill("76 00 00 98");
  await page.getByRole("button", { name: texte("messages.essai_envoyer") }).click();
  const ligne = page.getByRole("row").filter({ hasText: "+223 76 00 00 98" });
  // L'essai passe devant les relances du jour, mais attend la minute suivante si le débit de la
  // minute est déjà pris.
  await expect(ligne.getByText(texte("messages.etat_message.remis"))).toBeVisible({ timeout: 75_000 });
  // Le crédit est lu au premier passage par la clé, avant l'envoi de l'essai.
  await expect(page.getByText(/Crédit de la puce : 5\s000\sF/)).toBeVisible();
  await capturer("122-cle-modem");
});

test("phase 5 : un bulletin de la 7e A se remet à l'oncle de l'élève ; ceux du 1er trimestre sont non retirés", async () => {
  await allerA("nav.remises");
  await page.locator("#remises-classe").selectOption({ label: "7e A" });
  await page.locator("#remises-periode").selectOption({ label: "3e trimestre" });
  const premiere = page.locator("tbody tr").first();
  await expect(premiere).toContainText(texte("remises.statut.a_remettre"));
  await premiere.click();
  await page.locator("[id^=remis-a-]").selectOption("tiers");
  await page.locator("[id^=remis-nom-]").fill("Bakary Diallo, son oncle");
  await page.locator("[id^=remis-signe-]").uncheck();
  await page.getByRole("button", { name: texte("remises.remettre") }).click();
  await expect(page.locator("tbody tr").first()).toContainText(`${texte("remises.statut.remis")} · Bakary Diallo, son oncle`);
  // Les bulletins du 1er trimestre, jamais remis, sont non retirés depuis la publication du 2e.
  await page.locator("#remises-periode").selectOption({ label: "1er trimestre" });
  await expect(page.locator("tbody tr").first()).toContainText(texte("remises.statut.non_retire"));
  await capturer("123-remise-des-bulletins");
});

test("phase 5 : un responsable est convoqué par le billet confié à l'élève ; sa réception se note", async () => {
  await allerA("nav.convocations");
  // L'élève se retrouve par quelques lettres de son nom, comme au guichet.
  await page.locator("#convocation-eleve").fill("Fatoumata");
  await page.getByRole("button", { name: /^Fatoumata Diallo \(/ }).first().click();
  await expect(page.getByRole("button", { name: texte("convocations.autre_eleve") })).toBeVisible();
  await page.locator("#convocation-motif").fill("Travail insuffisant en mathématiques");
  await page.locator("#convocation-date").fill("2026-10-20");
  await page.locator("#convocation-heure").fill("10:30");
  await page.locator("#convocation-canal").selectOption("eleve");
  await page.getByRole("button", { name: texte("convocations.convoquer") }).click();
  const ligne = page.getByRole("row").filter({ hasText: "Travail insuffisant en mathématiques" });
  await expect(ligne).toContainText(texte("convocations.accuse.transmis_non_confirme"));
  await page.getByRole("button", { name: texte("convocations.noter").replace("{accuse}", texte("convocations.accuse.confirme")) }).click();
  await expect(ligne).toContainText(texte("convocations.accuse.confirme"));
  await capturer("124-convocations");
  await page.getByRole("button", { name: texte("convocations.imprimer") }).click();
  await expect(page.locator("iframe")).toBeVisible({ timeout: 30_000 });
});

test("phase 5 : au guichet, la mère de Fatoumata voit toute sa famille, ce qu'elle doit et ce qui l'attend", async () => {
  await allerA("nav.guichet");
  await page.locator("#guichet-recherche").fill("Fatoumata Diallo");
  await page.getByRole("button", { name: /^Fatoumata Diallo · / }).first().click();
  await expect(page.getByRole("heading", { name: texte("guichet.famille"), exact: true })).toBeVisible();
  await expect(page.getByText("Kadiatou Diallo")).toBeVisible();
  await expect(page.getByRole("row").filter({ hasText: "Fatoumata Diallo" })).toBeVisible();
  await expect(page.getByText(/La famille doit d'échu/)).toBeVisible();
  // Ses bulletins des 1er et 2e trimestres n'ont jamais été retirés.
  await expect(page.getByText(texte("guichet.bulletin").replace("{eleve}", "Fatoumata Diallo").replace("{periode}", "1er trimestre"))).toBeVisible();
  await capturer("125-guichet");
});

test("phase 5 : la mère de Fatoumata reçoit son code au guichet ; elle demande son solde par SMS et la réponse part", async () => {
  await allerA("nav.parametres");
  await page.locator('[id="param-sms.consultation_active"]').check();
  await page.locator('[id="param-sms.consultation_active"]').locator("xpath=ancestor::div[contains(@class,'py-2')][1]").getByRole("button", { name: texte("commun.enregistrer") }).click();
  await expect(page.locator('[id="param-sms.consultation_active"]')).toBeChecked();

  await allerA("nav.guichet");
  await page.locator("#guichet-recherche").fill("Fatoumata Diallo");
  await page.getByRole("button", { name: /^Fatoumata Diallo · / }).first().click();
  const mere = page.locator("li").filter({ hasText: "Kadiatou Diallo" }).first();
  await expect(mere.getByText(texte("guichet.code.aucun"))).toBeVisible();
  await mere.getByRole("button", { name: texte("guichet.code.remettre") }).click();
  const code = page.getByTestId("code-acces");
  await expect(code).toHaveText(/^[A-HJ-NP-Z2-9]{3} [A-HJ-NP-Z2-9]{3}$/);
  await expect(page.locator("iframe")).toBeVisible({ timeout: 30_000 });
  await expect(mere.getByRole("button", { name: texte("guichet.code.remplacer") })).toBeVisible();
  await capturer("126-code-acces");
  const saisi = ((await code.textContent()) ?? "").replace(" ", "");
  const telephone = /\+223 \d\d \d\d \d\d \d\d/.exec((await mere.textContent()) ?? "")?.[0] ?? "";
  expect(telephone).not.toBe("");

  await allerA("nav.messages");
  await page.locator("#sms-simule-numero").fill(telephone);
  await page.locator("#sms-simule-texte").fill(`solde ${saisi.toLowerCase()}`);
  await page.getByRole("button", { name: texte("messages.simuler_envoyer") }).click();
  // La file passe toutes les cinq secondes : elle reçoit le SMS, puis la réponse part.
  const recu = page.getByRole("row").filter({ hasText: "SOLDE ******" });
  await expect(recu.getByText(texte("messages.issue.servie"))).toBeVisible({ timeout: 20_000 });
  await expect(recu).toContainText("Kadiatou Diallo");
  await expect(recu).toContainText("reste a payer Fatoumata");
  await capturer("127-sms-recus");
});

test("le programme de la 9e s'importe d'un fichier corrigé, rangé dans les paramètres ; le bambara entre au programme", async () => {
  const fichier = join(dossier, "programme-9e.csv");
  const lignes = [
    "Niveau;Série;Matière;Coefficient;Heures par semaine",
    "9e année;;Français;4;6",
    "9e année;;Mathématiques;4;5",
    "9e année;;Physique-chimie;2;2",
    "9e année;;Sciences de la vie et de la terre;2;2",
    "9e année;;Histoire-géographie;2;3",
    "9e année;;Anglais;2;3",
    "9e année;;Bambara;1;1h30",
    "9e année;;Éducation physique et sportive;1;2",
  ];
  writeFileSync(fichier, `${String.fromCharCode(0xfeff)}${lignes.join("\r\n")}\r\n`, "utf8");
  await allerA("nav.parametres");
  await page.getByText(texte("import_programme.titre")).click();
  await page.evaluate((chemin) => {
    const fenetre = window as unknown as { fetchOriginal?: typeof fetch };
    const original = window.fetch.bind(window);
    fenetre.fetchOriginal = original;
    window.fetch = (entree, options) => {
      const url = decodeURIComponent(typeof entree === "string" ? entree : entree instanceof URL ? entree.href : entree.url);
      if (url.includes("ipc.localhost/plugin:dialog|open")) {
        return Promise.resolve(new Response(JSON.stringify(chemin), { status: 200, headers: { "Tauri-Response": "ok", "content-type": "application/json" } }));
      }
      return original(entree, options);
    };
  }, fichier);
  await page.getByRole("button", { name: texte("import_programme.choisir") }).click();
  await expect(page.getByText("1 niveau(x) prêt(s) ; 0 laissé(s) tel(s) quel(s) à cause d'une ligne à corriger.")).toBeVisible();
  await page.evaluate(() => {
    const fenetre = window as unknown as { fetchOriginal?: typeof fetch };
    if (fenetre.fetchOriginal !== undefined) window.fetch = fenetre.fetchOriginal;
  });
  await expect(page.getByText(texte("import_programme.nouvelles").replace("{matieres}", "Bambara"))).toBeVisible();
  await capturer("129-import-du-programme");
  await page.getByRole("button", { name: "Importer le programme de 1 niveau(x)" }).click();
  // L'import commence par une sauvegarde.
  await expect(page.getByText("Programme importé : 1 niveau(x) récrit(s), 1 matière(s) créée(s), 0 niveau(x) laissé(s) tel(s) quel(s).")).toBeVisible({ timeout: 30_000 });
});

test("les classes et les matières s'importent d'un classeur, sans toucher au programme", async () => {
  const fichier = join(dossier, "classes-et-matieres.xlsx");
  copyFileSync(join(import.meta.dirname, "..", "..", "docs", "exemples", "classes-et-matieres.xlsx"), fichier);
  await allerA("nav.parametres");
  await page.getByText(texte("import_referentiel.titre")).click();
  await page.evaluate((chemin) => {
    const fenetre = window as unknown as { fetchOriginal?: typeof fetch };
    const original = window.fetch.bind(window);
    fenetre.fetchOriginal = original;
    window.fetch = (entree, options) => {
      const url = decodeURIComponent(typeof entree === "string" ? entree : entree instanceof URL ? entree.href : entree.url);
      if (url.includes("ipc.localhost/plugin:dialog|open")) {
        return Promise.resolve(new Response(JSON.stringify(chemin), { status: 200, headers: { "Tauri-Response": "ok", "content-type": "application/json" } }));
      }
      return original(entree, options);
    };
  }, fichier);
  await page.getByRole("button", { name: texte("import_referentiel.choisir") }).click();
  await expect(page.getByText(/classe\(s\) et \d+ matière\(s\) à créer\./)).toBeVisible();
  await page.evaluate(() => {
    const fenetre = window as unknown as { fetchOriginal?: typeof fetch };
    if (fenetre.fetchOriginal !== undefined) window.fetch = fenetre.fetchOriginal;
  });
  // Les classes que l'école a déjà sont laissées telles quelles.
  await expect(page.getByRole("row").filter({ has: page.getByRole("cell", { name: "7e A", exact: true }) }).getByText(texte("import_referentiel.deja"))).toBeVisible();
  await capturer("130-import-classes-et-matieres");
  await page.getByRole("button", { name: /^Créer \d+ classe\(s\) et matière\(s\)$/ }).click();
  await expect(page.getByText(/^Import terminé : \d+ classe\(s\) et \d+ matière\(s\) créée\(s\)/)).toBeVisible({ timeout: 30_000 });
});

test("version 1.2.2 : un parent payé le 10 voit les échéances de son enfant passer au 10", async () => {
  await allerA("nav.eleves");
  await page.locator("#eleves-recherche").fill("Tenin TANGARA");
  await page.getByRole("cell", { name: "Tenin" }).first().click();
  await expect(page.getByRole("heading", { name: "Tenin TANGARA" })).toBeVisible();
  await page.locator('[id^="jour-"]').first().fill("10");
  await page.getByRole("button", { name: texte("facture.appliquer_jour") }).first().click();
  await expect(page.getByRole("row").filter({ hasText: "Scolarité · 2e tranche" }).getByText("10 janvier 2027")).toBeVisible();
  await page.getByRole("row").filter({ hasText: "Scolarité · 2e tranche" }).scrollIntoViewIfNeeded();
  await capturer("131-jour-echeance-eleve");
});

test("version 1.2.4 : l'école active la note de conduite calculée ; le blâme du conseil la fait baisser, la direction en corrige une, le bulletin la compte", async () => {
  await allerA("nav.parametres");
  await page.locator("#module-conduite").check();
  await expect(page.getByRole("navigation").getByRole("button", { name: texte("nav.conduite"), exact: true })).toBeVisible();
  const mode = page.locator('[id="param-conduite.mode"]');
  await mode.selectOption("automatique");
  await mode.locator("xpath=ancestor::div[contains(@class,'py-2')][1]").getByRole("button", { name: texte("commun.enregistrer") }).click();
  await expect(mode.locator("xpath=ancestor::div[contains(@class,'py-2')][1]").getByText(/Enregistré/)).toBeVisible();
  await page.getByRole("heading", { name: texte("parametres.theme.conduite") }).scrollIntoViewIfNeeded();
  await capturer("133-reglages-conduite");

  // En 7e A, Fatoumata a le blâme du conseil (4 points) et trois jours d'exclusion temporaire
  // (5 points) au 1er trimestre : 11 sur 20.
  await allerA("nav.conduite");
  await page.locator("#conduite-classe").selectOption({ label: "7e A" });
  await page.locator("#conduite-periode").selectOption({ label: "1er trimestre" });
  const fatoumata = page.getByRole("row").filter({ hasText: "Fatoumata Diallo" });
  await expect(fatoumata.getByText("1 blâme(s) · 1 exclusion(s)")).toBeVisible();
  await expect(fatoumata.getByRole("cell").nth(2)).toHaveText("11,00");
  // Corriger la note calculée demande un motif ; la direction le donne.
  await fatoumata.locator("#conduite-note-1").fill("13");
  await fatoumata.getByRole("button", { name: texte("commun.enregistrer") }).click();
  await expect(fatoumata.getByText(texte("conduite.motif_obligatoire"))).toBeVisible();
  await fatoumata.locator("#conduite-motif-1").fill("Excuses présentées au conseil de discipline");
  await fatoumata.getByRole("button", { name: texte("commun.enregistrer") }).click();
  await expect(fatoumata.getByText("corrigée : Excuses présentées au conseil de discipline")).toBeVisible();
  await expect(fatoumata.getByRole("cell").nth(3)).toContainText("13,00");
  await capturer("134-note-de-conduite");

  // En 7e B, les bulletins du 1er trimestre la portent comme une matière, au coefficient 1.
  await allerA("nav.bulletins");
  await page.locator("#bulletins-classe").selectOption({ label: "7e B · 7ème année" });
  await page.locator("#bulletins-periode").selectOption({ label: "1er trimestre" });
  await page.getByRole("button", { name: texte("bulletins.calculer") }).click();
  await expect(page.getByText(/^Bulletins calculés : \d+\.$/)).toBeVisible({ timeout: 60_000 });
  await page.getByRole("button", { name: texte("bulletins.ouvrir") }).first().click();
  const conduite = page.getByRole("row").filter({ hasText: texte("bulletins.conduite") });
  await expect(conduite).toBeVisible();
  await expect(conduite.getByRole("cell").nth(4)).toHaveText("1");
  await conduite.scrollIntoViewIfNeeded();
  await capturer("135-bulletin-ecran-conduite");
  await page.getByRole("button", { name: texte("bulletins.imprimer_un") }).click();
  await expect(page.locator("iframe")).toBeVisible({ timeout: 30_000 });
  // Le visionneur PDF de WebView2 rend la page hors du DOM : aucun signal à attendre.
  await page.waitForTimeout(4000);
  await page.locator("iframe").scrollIntoViewIfNeeded();
  await capturer("136-bulletin-imprime-conduite");
});

test("version 1.2.5 : Ctrl + R garde la session ; l'école élargit la colonne des matières, le directeur et le parent signent en gras", async () => {
  await page.reload();
  await expect(page.getByRole("navigation").getByRole("button", { name: texte("nav.accueil"), exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: texte("connexion.valider") })).toHaveCount(0);

  await allerA("nav.parametres");
  const bloc = page.locator("section").filter({ has: page.getByRole("heading", { name: texte("colonnes.titre") }) });
  await bloc.locator("#colonnes-document").selectOption("bulletin");
  await bloc.locator('[id="colonne-colonnes.bulletin.matiere"]').fill("70");
  await bloc.getByRole("button", { name: texte("commun.enregistrer") }).click();
  await expect(bloc.getByText(/Enregistré/)).toBeVisible();
  await bloc.scrollIntoViewIfNeeded();
  await capturer("137-largeur-des-colonnes");

  await allerA("nav.bulletins");
  await page.locator("#bulletins-classe").selectOption({ label: "7e B · 7ème année" });
  await page.locator("#bulletins-periode").selectOption({ label: "1er trimestre" });
  await page.getByRole("button", { name: texte("bulletins.ouvrir") }).first().click();
  await page.getByRole("button", { name: texte("bulletins.imprimer_un") }).click();
  await expect(page.locator("iframe")).toBeVisible({ timeout: 30_000 });
  // Le visionneur PDF de WebView2 rend la page hors du DOM : aucun signal à attendre.
  await page.waitForTimeout(4000);
  await page.locator("iframe").scrollIntoViewIfNeeded();
  await capturer("138-bulletin-colonnes-et-signatures");
});
