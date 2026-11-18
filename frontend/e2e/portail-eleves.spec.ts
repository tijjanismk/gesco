// Phase 12, lot 4, sur la vraie application : la direction allume le réseau puis le portail des
// élèves, le secrétariat imprime les billets de la 9e A, et un « téléphone » de 360 pixels (Edge
// du poste, piloté par Playwright : aucun navigateur téléchargé) lit sans code, puis entre avec
// un code, lit une leçon avec formules et son, la marque finie et enregistre son PDF ; le maître
// de la classe le voit au suivi. Module E-learning éteint : le portail se tait.
//
// Prérequis : `pnpm build:eleve` (lancé ici s'il manque), le paquet signé de la bibliothèque
// (`GESCO_PAQUETS_BIBLIOTHEQUE`), `gesco-app` compilé en développement. En mode d'essai, le portail
// écoute le port 47 801 de la machine seule, et les billets déposent leurs codes dans
// `codes-portail-essai.json` du dossier d'essai : le code d'un vrai élève n'est écrit que sur son
// billet.
import { type ChildProcess, spawn, spawnSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync } from "node:fs";
import { connect } from "node:net";
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
/** Rentrée et fin de l'année 2026-2027, en millisecondes. */
const RENTREE_2026 = 1_790_812_800_000;
const FIN_2027 = 1_816_992_000_000;
/** Élèves de la 9e A : prénom et matricule. */
const ELEVES: [string, string][] = [
  ["Adama", "E-901"],
  ["Kadiatou", "E-902"],
  ["Seydou", "E-903"],
];

const texte = (cle: Cle, valeurs: Record<string, string | number> = {}) =>
  catalogue[cle].replace(/\{(\w+)\}/g, (entier, nom: string) => (valeurs[nom] === undefined ? entier : String(valeurs[nom])));

/** Textes des écrans de l'élève (`frontend/eleve/src/textes.ts`). */
const ELEVE = {
  lireSansCode: "Lire sans code",
  avancementPasGarde: "Ton avancement n'est pas gardé.",
  matricule: "Matricule",
  code: "Code",
  entrer: "Entrer",
  refusIncorrect: "Matricule ou code incorrect. Demande ton code au secrétariat.",
  mesMatieres: "Mes matières",
  jaiFini: "J'ai fini cette leçon",
  cestNote: "Bravo, c'est noté.",
  enregistrerPdf: "Enregistrer en PDF",
  monAvancement: "Mon avancement",
  sortir: "Sortir",
};

let dossier: string;
let donnees: string;
let processus: ChildProcess;
let application: Browser;
let page: Page;
let edge: Browser;

async function lancer(portDebogage: number) {
  donnees = join(dossier, "donnees");
  // Le paquet signé de la bibliothèque, déposé comme le fait l'installeur.
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
    for (const groupe of ["nav.groupe.parametres", "nav.groupe.outils"] as const) {
      const entete = navigation.getByRole("button", { name: texte(groupe), exact: true });
      if ((await entete.count()) > 0 && (await entete.getAttribute("aria-expanded")) === "false") await entete.click();
    }
  }
  await bouton.click();
}

/** Appelle des commandes de l'application depuis la page, avec la session ouverte : `api.appeler`
 *  dans `script`, qui rend sa valeur. Sert à préparer l'année, la classe et ses élèves. */
async function parCommandes<T>(script: string): Promise<T> {
  return (await page.evaluate(
    `(async () => { const api = await import("/src/api/index.ts"); api.definirJeton(window.sessionStorage.getItem("gesco.jeton")); ${script} })()`,
  )) as T;
}

/** Vrai si un programme écoute le port du portail sur cette machine. */
function portailEcoute(): Promise<boolean> {
  return new Promise((resoudre) => {
    const prise = connect({ host: "127.0.0.1", port: PORT_PORTAIL });
    prise.once("connect", () => {
      prise.destroy();
      resoudre(true);
    });
    prise.once("error", () => resoudre(false));
  });
}

/** Codes déposés par les billets en mode d'essai : matricule → code. */
function codesDeposes(): Record<string, string> {
  const chemin = join(donnees, "codes-portail-essai.json");
  return existsSync(chemin) ? (JSON.parse(readFileSync(chemin, "utf8")) as Record<string, string>) : {};
}

/** Un téléphone de 360 pixels, tactile, avec son propre témoin de session. */
function telephone(): Promise<BrowserContext> {
  return edge.newContext({
    viewport: { width: 360, height: 740 },
    deviceScaleFactor: 2,
    isMobile: true,
    hasTouch: true,
    locale: "fr-FR",
    acceptDownloads: true,
  });
}

/** Aucun défilement de côté : l'écran se lit sur la largeur du téléphone. */
async function tientEnLargeur(p: Page) {
  const largeur = await p.evaluate(() => document.documentElement.scrollWidth);
  expect(largeur).toBeLessThanOrEqual(360);
}

/** Des matières à la leçon de la racine carrée, sur le téléphone. */
async function ouvrirRacineCarree(p: Page) {
  await expect(p.getByRole("heading", { name: ELEVE.mesMatieres })).toBeVisible({ timeout: 30_000 });
  await p.getByRole("link", { name: /Mathématiques/ }).first().click();
  await p.getByRole("link", { name: new RegExp(RACINE_CARREE) }).first().click();
  await expect(p.getByRole("heading", { name: RACINE_CARREE })).toBeVisible({ timeout: 30_000 });
  // Les formules sont rendues par KaTeX, comme au bureau.
  await expect(p.locator(".katex").first()).toBeVisible({ timeout: 30_000 });
}

test.describe.configure({ mode: "serial" });

test.beforeAll(async () => {
  test.setTimeout(300_000);
  // Les écrans de l'élève, construits comme avant l'installeur.
  if (!existsSync(PAGES_ELEVE)) {
    const construction = spawnSync("pnpm", ["build:eleve"], { cwd: join(import.meta.dirname, ".."), shell: true, stdio: "inherit" });
    if (construction.status !== 0) throw new Error("pnpm build:eleve a échoué");
  }
  dossier = mkdtempSync(join(tmpdir(), "gesco-portail-"));
  await lancer(9227);
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

test("lot 4 : l'école est créée avec la bibliothèque, une 9e A de trois élèves et son maître de mathématiques", async () => {
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

  // L'année, ses frais, la 9e A et ses élèves, le maître de mathématiques : par les commandes.
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
    let rang = 0;
    for (const [prenom, matricule] of ${eleves}) {
      rang += 1;
      const fiche = await api.appeler("eleve_creer", { identite: { nom: "Coulibaly", prenoms: prenom, sexe: rang % 2 === 0 ? "feminin" : "masculin",
        date_naissance: "2011-01-05", date_naissance_estimee: false, lieu_naissance: "Ségou", nationalite: "Malienne", adresse: "", telephones: [] },
        matricule_interne: matricule, matricule_national: "MN-90000" + String(rang), acte_nature: "acte", acte_numero: "", acte_date: "", ecole_origine: "" });
      await api.appeler("inscription_creer", { eleve_id: fiche.eleve.id, classe_id: classe.id, date_inscription: "", redoublant: false, etablissement_origine: "" });
    }
    const bilan = await api.appeler("inscriptions_valider_classe", { id: classe.id });
    const compte = await api.appeler("utilisateur_creer", { identifiant: "sekou", nom_affiche: "Sékou Traoré", mot_de_passe_initial: "enseignant-2026", roles: ["enseignant"] });
    const maitre = await api.appeler("enseignant_enregistrer", { id: null, nom: "Traoré", prenoms: "Sékou", telephones: [], statut: "titulaire", utilisateur_id: compte.id, actif: true });
    const maths = ped.matieres.find((m) => m.code === "MATH");
    await api.appeler("affectation_ajouter", { classe_id: classe.id, matiere_id: maths.id, enseignant_id: maitre.enseignant.id });
    // Les maîtres ont leur compte : cours et suivi, sans la saisie des notes.
    await api.appeler("parametre_definir", { cle: "maitres.comptes", valeur: { type: "booleen", valeur: true } });
    return bilan.validees;
  `);
  expect(inscrits).toBe(ELEVES.length);
});

test("lot 4 : un son s'ajoute à la leçon de la racine carrée et se publie", async () => {
  test.setTimeout(240_000);
  await allerA("nav.bibliotheque");
  const champ = page.locator("#bibliotheque-recherche");
  await champ.fill("racine carree");
  await page.getByRole("button").filter({ hasText: RACINE_CARREE }).first().click();
  await page.getByRole("button", { name: texte("bibliotheque.mode.ecrire"), exact: true }).click();
  await page.getByRole("button", { name: texte("bibliotheque.editeur.commencer"), exact: true }).click();
  const editeur = page.locator("#bibliotheque-editeur");
  await expect(editeur).not.toHaveValue("");
  await editeur.press("Control+End");
  await editeur.pressSequentially("\n\nÉcoute la consigne :\n\n");
  // La boîte « Ouvrir » du système rend le son d'essai.
  const mp3 = join(dossier, "Consigne.mp3");
  copyFileSync(join(import.meta.dirname, "son-essai.mp3"), mp3);
  await page.evaluate((chemin) => {
    const original = window.fetch.bind(window);
    window.fetch = (entree, options) => {
      const url = decodeURIComponent(typeof entree === "string" ? entree : entree instanceof URL ? entree.href : entree.url);
      if (url.includes("ipc.localhost/plugin:dialog|open")) {
        window.fetch = original;
        return Promise.resolve(new Response(JSON.stringify(chemin), { status: 200, headers: { "Tauri-Response": "ok", "content-type": "application/json" } }));
      }
      return original(entree, options);
    };
  }, mp3);
  await page.getByRole("button", { name: texte("bibliotheque.editeur.son"), exact: true }).click();
  await expect(editeur).toHaveValue(/son:[0-9a-f]{64}/, { timeout: 30_000 });
  await page.getByRole("button", { name: texte("bibliotheque.editeur.enregistrer"), exact: true }).click();
  await page.getByRole("button", { name: texte("bibliotheque.editeur.publier"), exact: true }).first().click();
  await expect(page.getByRole("button", { name: texte("bibliotheque.editeur.commencer"), exact: true })).toBeVisible();
});

test("lot 4 : la direction allume le réseau puis le portail ; l'adresse et le pare-feu se lisent", async () => {
  test.setTimeout(120_000);
  // Éteint à la livraison : rien n'écoute.
  expect(await portailEcoute()).toBe(false);
  await allerA("nav.parametres");
  const section = page.locator("section").filter({ has: page.getByRole("heading", { name: texte("portail.reglages.titre"), exact: true }) });
  await section.locator("#portail-eleves").check();
  // Sans le réseau de l'école, une ligne et le bouton qui l'allume ; rien ne bloque.
  await expect(section.getByText(texte("portail.reglages.reseau_d_abord"))).toBeVisible();
  await section.getByRole("button", { name: texte("portail.reglages.allumer_reseau"), exact: true }).click();
  await expect(section.getByTestId("adresse-portail")).toHaveText(ADRESSE, { timeout: 30_000 });
  expect(await portailEcoute()).toBe(true);
  // Le pare-feu simulé refuse la première demande, comme une personne qui répond « Non » à
  // Windows, puis accepte : la ligne disparaît.
  const ouvrir = section.getByRole("button", { name: texte("postes.pare_feu.ouvrir"), exact: true });
  await expect(section.getByText(texte("portail.reglages.pare_feu"))).toBeVisible();
  await ouvrir.click();
  await expect(ouvrir).toBeEnabled();
  await ouvrir.click();
  await expect(section.getByText(texte("portail.reglages.pare_feu"))).toBeHidden({ timeout: 15_000 });
  await section.screenshot({ path: join(CAPTURES, "400-lot4-portail-allume.png") });
});

test("lot 4 : le secrétariat imprime les billets de la 9e A ; chaque élève a son code", async () => {
  test.setTimeout(120_000);
  await allerA("nav.classes");
  await page.getByRole("row").filter({ hasText: "9e A" }).first().click();
  const codes = page.locator("section").filter({ has: page.getByRole("heading", { name: texte("portail.codes.titre"), exact: true }) });
  await expect(codes.getByText(texte("portail.codes.compte", { avec: 0, eleves: ELEVES.length }))).toBeVisible();
  await codes.getByRole("button", { name: texte("portail.codes.sans_code"), exact: true }).click();
  await expect(codes.getByText(texte("portail.codes.compte", { avec: ELEVES.length, eleves: ELEVES.length }))).toBeVisible({ timeout: 30_000 });
  await capturer(page, "401-lot4-billets");
  const deposes = codesDeposes();
  for (const [, matricule] of ELEVES) expect(deposes[matricule]).toMatch(/^[A-Z0-9]{3} [A-Z0-9]{3}$/);
  // Une seconde fois : tous ont leur code, rien à imprimer.
  await codes.getByRole("button", { name: texte("portail.codes.sans_code"), exact: true }).click();
  await expect(codes.getByText(texte("portail.codes.aucun_a_faire"))).toBeVisible();
});

test("lot 4 : un téléphone de 360 pixels lit sans code ; rien n'est gardé", async () => {
  test.setTimeout(180_000);
  const contexte = await telephone();
  const p = await contexte.newPage();
  await p.goto(ADRESSE);
  await expect(p.getByRole("button", { name: ELEVE.lireSansCode })).toBeVisible({ timeout: 30_000 });
  await tientEnLargeur(p);
  await capturer(p, "402-lot4-telephone-entree");
  await p.getByRole("button", { name: ELEVE.lireSansCode }).click();
  await expect(p.getByText(ELEVE.avancementPasGarde)).toBeVisible();
  await p.getByRole("button", { name: /9e/ }).first().click();
  await ouvrirRacineCarree(p);
  await tientEnLargeur(p);
  // Sans code : ni « J'ai fini », ni « Mon avancement ».
  await expect(p.getByRole("button", { name: ELEVE.jaiFini })).toHaveCount(0);
  await capturer(p, "403-lot4-telephone-lecon-sans-code");
  await contexte.close();
});

test("lot 4 : un élève entre avec son code, écoute le son, finit la leçon et enregistre son PDF", async () => {
  test.setTimeout(240_000);
  const [prenom, matricule] = ELEVES[0] ?? ["", ""];
  const code = codesDeposes()[matricule] ?? "";
  const contexte = await telephone();
  const p = await contexte.newPage();
  await p.goto(ADRESSE);
  // Un code faux : la même réponse que pour un matricule inconnu.
  await p.getByLabel(ELEVE.matricule).fill(matricule);
  await p.getByLabel(ELEVE.code).fill("ZZZ ZZZ");
  await p.getByRole("button", { name: ELEVE.entrer, exact: true }).click();
  await expect(p.getByRole("alert")).toHaveText(ELEVE.refusIncorrect);
  // Le bon code, tapé en minuscules.
  await p.getByLabel(ELEVE.code).fill(code.toLowerCase());
  await p.getByRole("button", { name: ELEVE.entrer, exact: true }).click();
  await expect(p.getByText(prenom).first()).toBeVisible({ timeout: 30_000 });
  await expect(p.getByText("9e A").first()).toBeVisible();
  await tientEnLargeur(p);
  await capturer(p, "404-lot4-telephone-mes-matieres");

  await ouvrirRacineCarree(p);
  // Le son se lit depuis le portail, par requêtes partielles : trois secondes.
  const son = p.locator("audio").first();
  await expect(son).toBeAttached({ timeout: 30_000 });
  await expect
    .poll(() => son.evaluate((a: HTMLAudioElement) => (Number.isFinite(a.duration) ? Math.round(a.duration) : 0)), { timeout: 30_000 })
    .toBe(3);
  await capturer(p, "405-lot4-telephone-lecon");

  // « J'ai fini cette leçon » : noté.
  await p.getByRole("button", { name: ELEVE.jaiFini }).click();
  await expect(p.getByText(ELEVE.cestNote)).toBeVisible();

  // Le PDF de la leçon se télécharge.
  const [telechargement] = await Promise.all([
    p.waitForEvent("download", { timeout: 120_000 }),
    p.getByRole("link", { name: ELEVE.enregistrerPdf }).click(),
  ]);
  const pdf = join(dossier, "lecon.pdf");
  await telechargement.saveAs(pdf);
  expect(telechargement.suggestedFilename()).toMatch(/\.pdf$/);
  expect(statSync(pdf).size).toBeGreaterThan(5_000);
  expect(readFileSync(pdf).subarray(0, 5).toString("latin1")).toBe("%PDF-");

  // Mon avancement : une leçon finie en mathématiques.
  await p.goto(`${ADRESSE}/#/avancement`);
  await expect(p.getByRole("heading", { name: ELEVE.monAvancement })).toBeVisible({ timeout: 30_000 });
  await expect(p.getByText(/^1 sur \d+ leçons? finies?$/).first()).toBeVisible();
  await tientEnLargeur(p);
  await capturer(p, "406-lot4-telephone-avancement");
  await contexte.close();
});

test("lot 4 : le maître de mathématiques voit au suivi la leçon finie par son élève", async () => {
  test.setTimeout(180_000);
  const [prenom] = ELEVES[0] ?? [""];
  // Le maître ouvre sa session depuis l'écran verrouillé, change son mot de passe.
  await page.getByRole("button", { name: texte("bandeau.verrouiller") }).click();
  await page.getByRole("button", { name: texte("verrou.autre_utilisateur") }).click();
  await page.getByLabel(texte("connexion.identifiant")).fill("sekou");
  await page.getByLabel(texte("connexion.mot_de_passe")).fill("enseignant-2026");
  await page.getByRole("button", { name: texte("connexion.valider") }).click();
  const changement = page.locator("form").first();
  await changement.getByLabel(texte("compte.mot_de_passe_actuel")).fill("enseignant-2026");
  await changement.getByLabel(texte("compte.nouveau_mot_de_passe")).fill("enseignant-2026-change");
  await changement.getByRole("button", { name: texte("commun.enregistrer") }).click();
  await expect(page.getByText(texte("bandeau.utilisateur", { nom: "Sékou Traoré" }))).toBeVisible({ timeout: 30_000 });

  await allerA("nav.bibliotheque");
  await page.getByRole("button", { name: texte("suivi.titre"), exact: true }).click();
  await page.getByRole("button", { name: texte("suivi.par_eleve"), exact: true }).click();
  // Les gestes s'écrivent toutes les cinq secondes, l'écran se relit toutes les dix.
  const ligne = page.getByRole("row").filter({ hasText: prenom });
  await expect(ligne).toBeVisible({ timeout: 30_000 });
  await expect(ligne).toContainText("1", { timeout: 30_000 });
  await capturer(page, "407-lot4-suivi-du-maitre");
});

test("lot 4 : module E-learning éteint, le portail se tait ; rallumé, il revient", async () => {
  test.setTimeout(120_000);
  // La direction reprend sa session ; le module se règle par sa commande.
  await page.getByRole("button", { name: texte("bandeau.verrouiller") }).click();
  await page.getByRole("button", { name: texte("verrou.autre_utilisateur") }).click();
  await page.getByLabel(texte("connexion.identifiant")).fill(IDENTIFIANT);
  await page.getByLabel(texte("connexion.mot_de_passe")).fill(NOUVEAU_MOT_DE_PASSE);
  await page.getByRole("button", { name: texte("connexion.valider") }).click();
  await expect(page.getByText(texte("bandeau.utilisateur", { nom: "Awa Traoré" }))).toBeVisible({ timeout: 30_000 });
  await parCommandes<unknown>(`return api.appeler("parametre_definir", { cle: "module.bibliotheque", valeur: { type: "booleen", valeur: false } });`);
  await expect.poll(portailEcoute, { timeout: 15_000 }).toBe(false);
  await parCommandes<unknown>(`return api.appeler("parametre_definir", { cle: "module.bibliotheque", valeur: { type: "booleen", valeur: true } });`);
  await expect.poll(portailEcoute, { timeout: 15_000 }).toBe(true);
});
