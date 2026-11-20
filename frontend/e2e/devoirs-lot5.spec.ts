// Phase 12, lots 3 et 5, sur la vraie application : un élève fait le quiz de la racine carrée sur
// un téléphone de 360 pixels et son maître le voit au suivi ; le maître donne un devoir à la 9e A,
// un élève le rend en trois photos prises « au téléphone », un autre en texte, un troisième sur
// papier ; le maître corrige, rend les copies ; l'élève lit sa note ; la direction reporte au
// bulletin, valide l'évaluation et calcule le bulletin. Module E-learning éteint, les devoirs
// disparaissent ; rallumé, ils reviennent.
//
// Prérequis : comme `portail-eleves.spec.ts` (pages de l'élève construites, paquet signé de la
// bibliothèque, `gesco-app` compilé en développement).
import { type ChildProcess, spawn, spawnSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { type Browser, type BrowserContext, type Page, chromium, expect, test } from "@playwright/test";

import { catalogue, type Cle } from "../src/i18n/fr-ML";

const EXECUTABLE = join(import.meta.dirname, "..", "..", "target", "debug", "gesco-app.exe");
const PAGES_ELEVE = join(import.meta.dirname, "..", "..", "crates", "gesco-app", "ressources", "portail-eleve", "index.html");
const CAPTURES = join(import.meta.dirname, "captures");
const IDENTIFIANT = "direction";
const MOT_DE_PASSE = "essai-premier-demarrage";
const NOUVEAU_MOT_DE_PASSE = "essai-mot-de-passe-change";
const RACINE_CARREE = "La racine carrée d'un nombre positif";
const PORT_PORTAIL = 47_801;
const ADRESSE = `http://127.0.0.1:${String(PORT_PORTAIL)}`;
const RENTREE_2026 = 1_790_812_800_000;
const FIN_2027 = 1_816_992_000_000;
const ELEVES: [string, string][] = [
  ["Adama", "E-901"],
  ["Kadiatou", "E-902"],
  ["Seydou", "E-903"],
];
const TITRE_DEVOIR = "Racines carrées : exercices 3 et 4";

const texte = (cle: Cle, valeurs: Record<string, string | number> = {}) =>
  catalogue[cle].replace(/\{(\w+)\}/g, (entier, nom: string) => (valeurs[nom] === undefined ? entier : String(valeurs[nom])));

/** Textes des écrans de l'élève (`frontend/eleve/src/textes.ts`). */
const ELEVE = {
  matricule: "Matricule",
  code: "Code",
  entrer: "Entrer",
  mesMatieres: "Mes matières",
  mesDevoirs: "Mes devoirs",
  faireLeQuiz: "Faire le quiz",
  verifier: "Vérifier",
  questionSuivante: "Question suivante",
  voirScore: "Voir mon score",
  recommencer: "Recommencer",
  ajouterPhoto: "Ajouter une photo",
  rendreMonDevoir: "Rendre mon devoir",
  taReponse: "Ta réponse",
  mesRendus: "Ce que tu as rendu",
  commentaireMaitre: "Mot du maître",
};

let dossier: string;
let donnees: string;
let processus: ChildProcess;
let application: Browser;
let page: Page;
let edge: Browser;

async function lancer(portDebogage: number) {
  donnees = join(dossier, "donnees");
  const paquets = process.env["GESCO_PAQUETS_BIBLIOTHEQUE"] ?? join(import.meta.dirname, "..", "..", "..", "..", "gesco-bibliotheque-paquets");
  mkdirSync(join(donnees, "bibliotheque"), { recursive: true });
  for (const fichier of readdirSync(paquets).filter((n) => n.endsWith(".gbib"))) {
    copyFileSync(join(paquets, fichier), join(donnees, "bibliotheque", fichier));
  }
  processus = spawn(EXECUTABLE, [], {
    env: {
      ...process.env,
      GESCO_DOSSIER_ESSAI: donnees,
      GESCO_DATE_ESSAI: "2026-10-14",
      GESCO_HEURE_ESSAI: "10",
      WEBVIEW2_USER_DATA_FOLDER: join(dossier, "webview"),
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${String(portDebogage)}`,
    },
    stdio: "ignore",
  });
  for (let essai = 0; essai < 60; essai += 1) {
    try {
      application = await chromium.connectOverCDP(`http://127.0.0.1:${String(portDebogage)}`);
      const [contexte] = application.contexts();
      if (contexte === undefined) throw new Error("WebView2 n'expose aucun contexte");
      page = contexte.pages()[0] ?? (await contexte.waitForEvent("page"));
      return;
    } catch {
      await new Promise((reprise) => setTimeout(reprise, 500));
    }
  }
  throw new Error(`WebView2 ne répond pas sur le port ${String(portDebogage)}`);
}

const DEPLIER = "html, body, #racine, .h-screen { height: auto !important; } main { overflow: visible !important; }";

async function capturer(p: Page, nom: string) {
  const depli = await p.addStyleTag({ content: DEPLIER });
  await p.screenshot({ path: join(CAPTURES, `${nom}.png`), fullPage: true });
  await depli.evaluate((style) => style.parentNode?.removeChild(style));
}

async function allerA(cle: Cle) {
  const navigation = page.getByRole("navigation").first();
  await navigation.getByRole("button", { name: texte("nav.accueil"), exact: true }).waitFor();
  const bouton = navigation.getByRole("button", { name: texte(cle), exact: true });
  if (!(await bouton.isVisible())) {
    for (const groupe of ["nav.groupe.scolarite", "nav.groupe.parametres", "nav.groupe.outils"] as const) {
      const entete = navigation.getByRole("button", { name: texte(groupe), exact: true });
      if ((await entete.count()) > 0 && (await entete.getAttribute("aria-expanded")) === "false") await entete.click();
    }
  }
  await bouton.click();
}

async function parCommandes<T>(script: string): Promise<T> {
  return (await page.evaluate(
    `(async () => { const api = await import("/src/api/index.ts"); api.definirJeton(window.sessionStorage.getItem("gesco.jeton")); ${script} })()`,
  )) as T;
}

function codesDeposes(): Record<string, string> {
  const chemin = join(donnees, "codes-portail-essai.json");
  return existsSync(chemin) ? (JSON.parse(readFileSync(chemin, "utf8")) as Record<string, string>) : {};
}

function telephone(): Promise<BrowserContext> {
  return edge.newContext({
    viewport: { width: 360, height: 740 },
    deviceScaleFactor: 2,
    isMobile: true,
    hasTouch: true,
    locale: "fr-FR",
  });
}

async function tientEnLargeur(p: Page) {
  const largeur = await p.evaluate(() => document.documentElement.scrollWidth);
  expect(largeur).toBeLessThanOrEqual(360);
}

/** L'élève de ce rang entre au portail avec son code, sur un téléphone neuf. */
async function entrer(rang: number): Promise<{ contexte: BrowserContext; p: Page }> {
  const [prenom, matricule] = ELEVES[rang] ?? ["", ""];
  const contexte = await telephone();
  const p = await contexte.newPage();
  await p.goto(ADRESSE);
  await p.getByLabel(ELEVE.matricule).fill(matricule);
  await p.getByLabel(ELEVE.code).fill(codesDeposes()[matricule] ?? "");
  await p.getByRole("button", { name: ELEVE.entrer, exact: true }).click();
  await expect(p.getByText(prenom).first()).toBeVisible({ timeout: 30_000 });
  return { contexte, p };
}

/** Ouvre une session au bureau ; change le mot de passe initial s'il est exigé. */
async function ouvrirSession(identifiant: string, motDePasse: string, nouveau: string | null, nom: string) {
  await page.getByRole("button", { name: texte("bandeau.verrouiller") }).click();
  await page.getByRole("button", { name: texte("verrou.autre_utilisateur") }).click();
  await page.getByLabel(texte("connexion.identifiant")).fill(identifiant);
  await page.getByLabel(texte("connexion.mot_de_passe")).fill(motDePasse);
  await page.getByRole("button", { name: texte("connexion.valider") }).click();
  if (nouveau !== null) {
    const changement = page.locator("form").first();
    await changement.getByLabel(texte("compte.mot_de_passe_actuel")).fill(motDePasse);
    await changement.getByLabel(texte("compte.nouveau_mot_de_passe")).fill(nouveau);
    await changement.getByRole("button", { name: texte("commun.enregistrer") }).click();
  }
  await expect(page.getByText(texte("bandeau.utilisateur", { nom }))).toBeVisible({ timeout: 30_000 });
}

/** Une photo de copie « prise au téléphone » : 3 000 × 4 000 pixels de bruit, en JPEG. */
async function photoDeTelephone(p: Page, graine: number): Promise<Buffer> {
  const base64 = await p.evaluate(async (g) => {
    const toile = document.createElement("canvas");
    toile.width = 3000;
    toile.height = 4000;
    const ctx = toile.getContext("2d");
    if (ctx === null) throw new Error("toile");
    const image = ctx.createImageData(toile.width, toile.height);
    let x = g * 2_654_435_761;
    for (let i = 0; i < image.data.length; i += 4) {
      x = (x * 1_103_515_245 + 12_345) >>> 0;
      const v = 200 + (x % 56);
      image.data[i] = v;
      image.data[i + 1] = v;
      image.data[i + 2] = v - (x % 20);
      image.data[i + 3] = 255;
    }
    ctx.putImageData(image, 0, 0);
    ctx.fillStyle = "#1a1a5e";
    ctx.font = "160px serif";
    ctx.fillText("√49 = 7", 300, 800);
    const blob = await new Promise<Blob>((resoudre, rejeter) => toile.toBlob((b) => (b === null ? rejeter(new Error("jpeg")) : resoudre(b)), "image/jpeg", 0.92));
    const octets = new Uint8Array(await blob.arrayBuffer());
    let binaire = "";
    for (let i = 0; i < octets.length; i += 0x8000) binaire += String.fromCharCode(...octets.subarray(i, i + 0x8000));
    return btoa(binaire);
  }, graine);
  return Buffer.from(base64, "base64");
}

test.describe.configure({ mode: "serial" });

test.beforeAll(async () => {
  test.setTimeout(300_000);
  if (!existsSync(PAGES_ELEVE)) {
    const construction = spawnSync("pnpm", ["build:eleve"], { cwd: join(import.meta.dirname, ".."), shell: true, stdio: "inherit" });
    if (construction.status !== 0) throw new Error("pnpm build:eleve a échoué");
  }
  dossier = mkdtempSync(join(tmpdir(), "gesco-devoirs-"));
  await lancer(9229);
  edge = await chromium.launch({ channel: "msedge" });
});

test.afterAll(async () => {
  await edge?.close().catch(() => undefined);
  await application?.close().catch(() => undefined);
  if (processus?.pid !== undefined) spawnSync("taskkill", ["/pid", String(processus.pid), "/T", "/F"]);
  try {
    rmSync(dossier, { recursive: true, force: true, maxRetries: 10, retryDelay: 500 });
  } catch {
    return;
  }
});

test("lots 3 et 5 : l'école, la 9e A, son maître de mathématiques, le portail et les codes", async () => {
  test.setTimeout(300_000);
  await expect(page.getByRole("heading", { name: texte("demarrage.titre") })).toBeVisible({ timeout: 30_000 });
  const champs: [Cle, string][] = [
    ["demarrage.organisation", "École du Fleuve"],
    ["demarrage.etablissement_nom", "École du Fleuve"],
    ["demarrage.code_academie", "AE-BKO-RD"],
    ["demarrage.code_cap", "CAP-01"],
    ["demarrage.adresse", "Bamako"],
    ["demarrage.telephone", "+223 20 22 00 00"],
    ["demarrage.administrateur_identifiant", IDENTIFIANT],
    ["demarrage.administrateur_nom", "Awa Traoré"],
    ["demarrage.administrateur_mot_de_passe", MOT_DE_PASSE],
  ];
  for (const [cle, valeur] of champs) await page.getByLabel(texte(cle)).fill(valeur);
  for (const cycle of ["premier_cycle", "second_cycle"]) await page.locator(`#cycle-${cycle}`).check();
  await page.locator("form").getByRole("button", { name: texte("demarrage.creer") }).click();
  await expect(page.getByRole("heading", { name: texte("phrase.titre") })).toBeVisible({ timeout: 90_000 });
  await page.getByLabel(texte("phrase.confirmation")).check();
  await page.getByRole("button", { name: texte("commun.continuer") }).click();
  await page.getByLabel(texte("connexion.identifiant")).fill(IDENTIFIANT);
  await page.getByLabel(texte("connexion.mot_de_passe")).fill(MOT_DE_PASSE);
  await page.getByRole("button", { name: texte("connexion.valider") }).click();
  const motDePasse = page.locator("form").first();
  await motDePasse.getByLabel(texte("compte.mot_de_passe_actuel")).fill(MOT_DE_PASSE);
  await motDePasse.getByLabel(texte("compte.nouveau_mot_de_passe")).fill(NOUVEAU_MOT_DE_PASSE);
  await motDePasse.getByRole("button", { name: texte("commun.enregistrer") }).click();
  await expect(page.getByText(texte("compte.changement_exige"))).toBeHidden();

  const eleves = JSON.stringify(ELEVES);
  const inscrits = await parCommandes<number>(`
    const annee = await api.appeler("annee_creer", { libelle: "2026-2027", debut: ${String(RENTREE_2026)}, fin: ${String(FIN_2027)} });
    await api.appeler("annee_activer", { id: annee.id });
    const referentiel = await api.appeler("niveaux_lister", null);
    const grille = await api.appeler("grille_creer", { annee_id: annee.id, copie_de: null });
    const lignes = referentiel.niveaux.flatMap((n) => [
      { niveau_id: n.id, serie_id: null, type_frais: "inscription", libelle: "Frais d'inscription", montant: 10000, obligatoire: true, exigibilite: "une_fois" },
      { niveau_id: n.id, serie_id: null, type_frais: "scolarite", libelle: "Scolarité", montant: 100000, obligatoire: true, exigibilite: "selon_plan" },
    ]);
    const plans = grille.contenu.plans.map((p) => ({ libelle: p.libelle, par_defaut: p.par_defaut, tranches: p.tranches }));
    await api.appeler("grille_modifier", { id: grille.grille.id, lue_le: grille.modifiee_le, libelle: grille.grille.libelle, lignes, plans });
    await api.appeler("grille_mettre_en_vigueur", { id: grille.grille.id });
    const ped = await api.appeler("pedagogie_lire", { id: annee.id });
    const neuvieme = ped.niveaux.find((n) => n.code === "9");
    const classe = await api.appeler("classe_creer", { annee_id: annee.id, niveau_id: neuvieme.id, serie_id: null, libelle: "9e A" });
    await api.appeler("parametre_definir", { cle: "matricule.saisie_manuelle", valeur: { type: "booleen", valeur: true } });
    let rang = 0;
    for (const [prenom, matricule] of ${eleves}) {
      rang += 1;
      const fiche = await api.appeler("eleve_creer", { identite: { nom: "Coulibaly", prenoms: prenom, sexe: rang % 2 === 0 ? "feminin" : "masculin",
        date_naissance: "2011-01-05", date_naissance_estimee: false, lieu_naissance: "Ségou", nationalite: "Malienne", adresse: "", telephones: [] },
        matricule_interne: matricule, matricule_national: "MN-90000" + String(rang), acte_nature: "acte", acte_numero: "", acte_date: "", ecole_origine: "" });
      await api.appeler("responsable_rattacher", { eleve_id: fiche.eleve.id, responsable_id: null,
        saisie: { identite: { nom: "Coulibaly", prenoms: "Mariam", sexe: "feminin", date_naissance: "", date_naissance_estimee: false,
          lieu_naissance: "", nationalite: "", adresse: "", telephones: ["76 12 34 0" + String(rang)] }, profession: "", employeur: "" },
        qualites: { lien: "mere", est_responsable_legal: true, est_payeur: true, recoit_notifications: true, vient_chercher: false, ordre_priorite: 1 } });
      await api.appeler("inscription_creer", { eleve_id: fiche.eleve.id, classe_id: classe.id, date_inscription: "", redoublant: false, etablissement_origine: "" });
    }
    const bilan = await api.appeler("inscriptions_valider_classe", { id: classe.id });
    const compte = await api.appeler("utilisateur_creer", { identifiant: "sekou", nom_affiche: "Sékou Traoré", mot_de_passe_initial: "enseignant-2026", roles: ["enseignant"] });
    const maitre = await api.appeler("enseignant_enregistrer", { id: null, nom: "Traoré", prenoms: "Sékou", telephones: [], statut: "titulaire", utilisateur_id: compte.id, actif: true });
    const maths = ped.matieres.find((m) => m.code === "MATH");
    await api.appeler("affectation_ajouter", { classe_id: classe.id, matiere_id: maths.id, enseignant_id: maitre.enseignant.id });
    await api.appeler("parametre_definir", { cle: "maitres.comptes", valeur: { type: "booleen", valeur: true } });
    return bilan.validees;
  `);
  expect(inscrits).toBe(ELEVES.length);

  // Le portail des élèves, allumé avec le réseau de l'école.
  await allerA("nav.parametres");
  const section = page.locator("section").filter({ has: page.getByRole("heading", { name: texte("portail.reglages.titre"), exact: true }) });
  await section.locator("#portail-eleves").click();
  await expect(section.locator("#portail-eleves")).toBeChecked();
  await section.getByRole("button", { name: texte("portail.reglages.allumer_reseau"), exact: true }).click();
  await expect(section.getByTestId("adresse-portail")).toHaveText(ADRESSE, { timeout: 30_000 });

  // Les billets de la 9e A : chaque élève a son code.
  await allerA("nav.classes");
  await page.getByRole("row").filter({ hasText: "9e A" }).first().click();
  const codes = page.locator("section").filter({ has: page.getByRole("heading", { name: texte("portail.codes.titre"), exact: true }) });
  await codes.getByRole("button", { name: texte("portail.codes.sans_code"), exact: true }).click();
  await expect(codes.getByText(texte("portail.codes.compte", { avec: ELEVES.length, eleves: ELEVES.length }))).toBeVisible({ timeout: 30_000 });
  for (const [, matricule] of ELEVES) expect(codesDeposes()[matricule]).toMatch(/^[A-Z0-9]{3} [A-Z0-9]{3}$/);
});

/** Répond à chaque question du quiz ouvert, jusqu'au score. */
async function passerLeQuiz(p: Page) {
  for (let tour = 0; tour < 30; tour += 1) {
    if (await p.getByRole("button", { name: ELEVE.recommencer }).isVisible()) return;
    const courte = p.getByLabel(ELEVE.taReponse);
    if ((await courte.count()) > 0 && (await courte.first().isEnabled())) await courte.first().fill("7");
    const listes = p.locator("select:enabled");
    for (let i = 0; i < (await listes.count()); i += 1) await listes.nth(i).selectOption({ index: 1 });
    const choix = p.locator("button[aria-pressed]:enabled");
    if ((await choix.count()) > 0) await choix.first().click();
    await p.getByRole("button", { name: ELEVE.verifier }).click();
    const suite = p.getByRole("button", { name: new RegExp(`^(${ELEVE.questionSuivante}|${ELEVE.voirScore})$`) });
    await suite.click();
  }
  throw new Error("le quiz ne finit pas");
}

test("lot 3 : deux élèves font le quiz de la racine carrée sur un téléphone de 360 pixels", async () => {
  test.setTimeout(300_000);
  for (const rang of [0, 1]) {
    const { contexte, p } = await entrer(rang);
    await p.getByRole("link", { name: /Mathématiques/ }).first().click();
    await p.getByRole("link", { name: new RegExp(RACINE_CARREE) }).first().click();
    await expect(p.getByRole("heading", { name: RACINE_CARREE })).toBeVisible({ timeout: 30_000 });
    await p.getByRole("link", { name: ELEVE.faireLeQuiz }).click();
    await expect(p.getByRole("button", { name: ELEVE.verifier })).toBeVisible({ timeout: 30_000 });
    await tientEnLargeur(p);
    if (rang === 0) await capturer(p, "500-lot3-telephone-quiz");
    await passerLeQuiz(p);
    await expect(p.getByText(/^\d+ sur \d+$/).first()).toBeVisible();
    if (rang === 0) await capturer(p, "501-lot3-telephone-score");
    // Les essais s'écrivent toutes les cinq secondes.
    await p.waitForTimeout(6_000);
    await contexte.close();
  }
});

test("lot 3 : le maître voit au suivi qui a fait le quiz et les questions manquées", async () => {
  test.setTimeout(180_000);
  await ouvrirSession("sekou", "enseignant-2026", "enseignant-2026-change", "Sékou Traoré");
  await allerA("nav.bibliotheque");
  await page.getByRole("button", { name: texte("suivi.titre"), exact: true }).click();
  await page.getByRole("button", { name: texte("suivi.quiz"), exact: true }).click({ timeout: 30_000 });
  await expect(page.getByText(texte("suivi.fait_par")).first()).toBeVisible({ timeout: 30_000 });
  await expect(page.getByText(/% de bonnes réponses/).first()).toBeVisible({ timeout: 30_000 });
  await capturer(page, "502-lot3-suivi-des-quiz");
  await page.getByRole("button", { name: texte("suivi.detail"), exact: true }).first().click();
  await expect(page.getByText(texte("suivi.premier_essai")).first()).toBeVisible();
  await capturer(page, "503-lot3-suivi-detail-du-quiz");
});

test("lot 5 : le maître donne un devoir à la 9e A", async () => {
  test.setTimeout(180_000);
  await allerA("nav.devoirs");
  await expect(page.getByText(texte("devoirs.aucun"))).toBeVisible({ timeout: 30_000 });
  await page.getByRole("button", { name: texte("devoirs.donner_un"), exact: true }).click();
  await page.locator("#devoir-titre").fill(TITRE_DEVOIR);
  await page
    .getByRole("textbox", { name: texte("devoirs.consignes"), exact: true })
    .fill("1. Calcule $\\sqrt{49}$ et $\\sqrt{81}$.\n\n2. Explique pourquoi $\\sqrt{-4}$ n'existe pas.\n\nPrends ta copie en photo, bien à plat.");
  await capturer(page, "504-lot5-donner-un-devoir");
  await page.getByRole("button", { name: texte("devoirs.creer"), exact: true }).click();
  await expect(page.getByText(texte("devoirs.brouillon_aide"))).toBeVisible({ timeout: 30_000 });
  await page.getByRole("button", { name: texte("devoirs.donner"), exact: true }).click();
  await expect(page.getByRole("button", { name: texte("devoirs.clore"), exact: true })).toBeVisible({ timeout: 30_000 });
  await capturer(page, "505-lot5-devoir-donne");
});

test("lot 5 : sur le téléphone, « Mes devoirs », trois photos rendues, puis un rendu en texte", async () => {
  test.setTimeout(300_000);
  const { contexte, p } = await entrer(0);
  await p.getByRole("link", { name: ELEVE.mesDevoirs }).click();
  await expect(p.getByRole("heading", { name: ELEVE.mesDevoirs })).toBeVisible({ timeout: 30_000 });
  await expect(p.getByText(TITRE_DEVOIR)).toBeVisible();
  await tientEnLargeur(p);
  await capturer(p, "506-lot5-telephone-mes-devoirs");
  await p.getByText(TITRE_DEVOIR).click();
  await expect(p.getByRole("button", { name: ELEVE.rendreMonDevoir })).toBeVisible({ timeout: 30_000 });
  const photos = [];
  for (const graine of [1, 2, 3]) {
    const octets = await photoDeTelephone(p, graine);
    expect(octets.length).toBeGreaterThan(1_000_000);
    photos.push({ name: `copie-${String(graine)}.jpg`, mimeType: "image/jpeg", buffer: octets });
  }
  await p.locator("input[type=file]").setInputFiles(photos);
  await expect(p.getByRole("img", { name: "Photo 3" })).toBeVisible({ timeout: 60_000 });
  await tientEnLargeur(p);
  await capturer(p, "507-lot5-telephone-trois-photos");
  await p.getByRole("button", { name: ELEVE.rendreMonDevoir }).click();
  await expect(p.getByText(/^Ton devoir est rendu le /)).toBeVisible({ timeout: 60_000 });
  await expect(p.getByText(ELEVE.mesRendus)).toBeVisible();
  await capturer(p, "508-lot5-telephone-devoir-rendu");
  await contexte.close();

  // Kadiatou rend en texte.
  const deux = await entrer(1);
  await deux.p.getByRole("link", { name: ELEVE.mesDevoirs }).click();
  await deux.p.getByText(TITRE_DEVOIR).click();
  await deux.p.getByLabel(ELEVE.taReponse).fill("√49 = 7 et √81 = 9. Un carré n'est jamais négatif, donc √−4 n'existe pas.");
  await deux.p.getByRole("button", { name: ELEVE.rendreMonDevoir }).click();
  await expect(deux.p.getByText(/^Ton devoir est rendu le /)).toBeVisible({ timeout: 60_000 });
  await deux.contexte.close();
});

test("lot 5 : le maître corrige, marque une copie sur papier et rend les copies", async () => {
  test.setTimeout(240_000);
  await allerA("nav.devoirs");
  await page.getByRole("button", { name: TITRE_DEVOIR, exact: true }).click();
  const adama = page.getByRole("row").filter({ hasText: "Adama" });
  await expect(adama).toBeVisible({ timeout: 30_000 });
  await capturer(page, "509-lot5-copies-des-eleves");
  await adama.getByRole("button", { name: texte("devoirs.ouvrir_copie"), exact: true }).click();
  // Chaque photo est arrivée réduite à 1 600 pixels au plus.
  await expect(page.locator("#copie-note")).toBeVisible({ timeout: 30_000 });
  await page.getByRole("button", { name: texte("devoirs.photo", { ordre: "1" }), exact: true }).click();
  for (const ordre of ["1", "2", "3"]) {
    const image = page.getByRole("img", { name: texte("devoirs.photo", { ordre }), exact: true });
    await expect(image).toBeVisible({ timeout: 30_000 });
    await expect.poll(() => image.evaluate((i: HTMLImageElement) => i.naturalWidth), { timeout: 30_000 }).toBeGreaterThan(0);
    const cote = await image.evaluate((i: HTMLImageElement) => Math.max(i.naturalWidth, i.naturalHeight));
    expect(cote).toBeLessThanOrEqual(1600);
    if (ordre === "1") await capturer(page, "510-lot5-photo-de-la-copie");
    if (ordre !== "3") await page.getByRole("button", { name: texte("devoirs.photo_suivante"), exact: true }).click();
  }
  await page.locator("#copie-note").fill("15,5");
  await page.getByLabel(texte("devoirs.commentaire")).fill("Juste. Soigne l'écriture de la racine.");
  await page.getByRole("button", { name: texte("devoirs.enregistrer_note"), exact: true }).click();
  await expect(page.getByText(texte("devoirs.note_enregistree"))).toBeVisible({ timeout: 30_000 });
  await capturer(page, "511-lot5-copie-corrigee");
  await page.getByRole("button", { name: texte("devoirs.eleve_suivant"), exact: true }).click();
  await page.locator("#copie-note").fill("12");
  await page.getByRole("button", { name: texte("devoirs.enregistrer_note"), exact: true }).click();
  await expect(page.getByText(texte("devoirs.note_enregistree"))).toBeVisible({ timeout: 30_000 });
  await page.getByRole("button", { name: texte("devoirs.fermer_copie"), exact: true }).click();
  // Seydou a remis sa copie à la main.
  const seydou = page.getByRole("row").filter({ hasText: "Seydou" });
  await seydou.getByRole("button", { name: texte("devoirs.rendu_papier"), exact: true }).click();
  await expect(seydou).toContainText(texte("devoirs.copie.sur_papier"), { timeout: 30_000 });
  await seydou.getByRole("button", { name: texte("devoirs.ouvrir_copie"), exact: true }).click();
  await page.locator("#copie-note").fill("9");
  await page.getByRole("button", { name: texte("devoirs.enregistrer_note"), exact: true }).click();
  await expect(page.getByText(texte("devoirs.note_enregistree"))).toBeVisible({ timeout: 30_000 });
  await page.getByRole("button", { name: texte("devoirs.fermer_copie"), exact: true }).click();
  await page.getByRole("button", { name: texte("devoirs.rendre_copies"), exact: true }).click();
  await expect(page.getByText(/^Copies rendues aux élèves le /)).toBeVisible({ timeout: 30_000 });
  // Sans la saisie des notes par les maîtres, la direction reporte.
  await expect(page.getByText(texte("devoirs.report_reserve"))).toBeVisible();
  await capturer(page, "512-lot5-copies-rendues");
});

test("lot 5 : l'élève lit sa note et le mot du maître, jamais ceux d'un autre", async () => {
  test.setTimeout(180_000);
  const { contexte, p } = await entrer(0);
  await p.getByRole("link", { name: ELEVE.mesDevoirs }).click();
  await p.getByText(TITRE_DEVOIR).click();
  await expect(p.getByText("15,5 sur 20")).toBeVisible({ timeout: 30_000 });
  await expect(p.getByText(ELEVE.commentaireMaitre)).toBeVisible();
  await expect(p.getByText("12 sur 20")).toHaveCount(0);
  await tientEnLargeur(p);
  await capturer(p, "513-lot5-telephone-note-et-mot-du-maitre");
  await contexte.close();
});

test("lot 5 : la direction reporte au bulletin, valide l'évaluation et calcule le bulletin", async () => {
  test.setTimeout(240_000);
  await ouvrirSession(IDENTIFIANT, NOUVEAU_MOT_DE_PASSE, null, "Awa Traoré");
  await allerA("nav.devoirs");
  await page.getByRole("button", { name: TITRE_DEVOIR, exact: true }).click();
  await expect(page.getByText(texte("devoirs.report_resume", { notes: 3, points: "20", ponderation: 1 }))).toBeVisible({ timeout: 30_000 });
  await page.getByRole("button", { name: texte("devoirs.reporter"), exact: true }).click();
  await expect(page.getByRole("button", { name: texte("devoirs.reporter_de_nouveau"), exact: true })).toBeVisible({ timeout: 30_000 });
  await capturer(page, "514-lot5-reporte-au-bulletin");

  // L'évaluation « devoir » porte les trois notes ; validée, le bulletin les compte.
  const bilan = await parCommandes<{ notes: number[]; moyenne: number | null; validee: boolean }>(`
    const devoirs = await api.appeler("devoirs_lister", {});
    const devoir = devoirs.devoirs[0];
    const report = await api.appeler("devoir_report", { id: devoir.id });
    const evaluation = report.evaluation.id;
    const grille = await api.appeler("notes_grille", { id: evaluation });
    await api.appeler("evaluation_valider", { id: evaluation });
    const calcul = await api.appeler("resultats_calculer", { classe_id: devoir.classe_id, periode_id: report.periode_id });
    const relu = await api.appeler("devoir_report", { id: devoir.id });
    const adama = calcul.bulletins.find((b) => JSON.stringify(b).includes("Adama"));
    const ligne = adama.bulletin.contenu.lignes.find((l) => l.moyenne !== null);
    return { notes: grille.lignes.map((l) => l.note?.valeur ?? null), moyenne: ligne === undefined ? null : ligne.moyenne, validee: relu.evaluation.validee };
  `);
  expect([...bilan.notes].sort((a, b) => a - b)).toEqual([900, 1200, 1550]);
  expect(bilan.validee).toBe(true);
  expect(bilan.moyenne).toBe(1550);
  await allerA("nav.devoirs");
  await page.getByRole("button", { name: TITRE_DEVOIR, exact: true }).click();
  await expect(page.getByText(texte("devoirs.evaluation_validee"))).toBeVisible({ timeout: 30_000 });
  await page.getByRole("button", { name: texte("devoirs.aller_aux_notes"), exact: true }).click();
  await expect(page.getByText(/Devoir rendu par les élèves/).first()).toBeVisible({ timeout: 30_000 });
  await capturer(page, "515-lot5-notes-devoir-rendu-par-les-eleves");
});

test("lot 5 : la place des photos se lit ; module éteint, les devoirs disparaissent puis reviennent", async () => {
  test.setTimeout(180_000);
  await allerA("nav.devoirs");
  await page.getByRole("button", { name: texte("devoirs.place"), exact: true }).click();
  await expect(page.getByText(texte("devoirs.place_ouverte"))).toBeVisible({ timeout: 30_000 });
  await capturer(page, "516-lot5-place-des-photos");
  await parCommandes<unknown>(`return api.appeler("parametre_definir", { cle: "module.bibliotheque", valeur: { type: "booleen", valeur: false } });`);
  const liste = await parCommandes<{ actif: boolean; devoirs: unknown[] }>(`return api.appeler("devoirs_lister", {});`);
  expect(liste.actif).toBe(false);
  expect(liste.devoirs).toHaveLength(0);
  await parCommandes<unknown>(`return api.appeler("parametre_definir", { cle: "module.bibliotheque", valeur: { type: "booleen", valeur: true } });`);
  const revenue = await parCommandes<{ actif: boolean; devoirs: { titre: string }[] }>(`return api.appeler("devoirs_lister", {});`);
  expect(revenue.actif).toBe(true);
  expect(revenue.devoirs.map((d) => d.titre)).toEqual([TITRE_DEVOIR]);
});
