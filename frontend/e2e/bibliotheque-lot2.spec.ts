// Phase 12, lot 2, sur la vraie application et deux écoles : l'école A ajoute un son à une leçon
// livrée et une leçon de l'école depuis une fiche Word, les publie et les exporte en un fichier
// `.gles` ; l'école B, avec le même paquet, importe ce fichier en brouillons, le réimporte sans
// rien ajouter, publie les leçons reçues et lit le son ; module éteint, rien de ce lot ne paraît.
// Chaque école a son espace de données et son dossier WebView2 jetables.
import { type ChildProcess, spawn, spawnSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readdirSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { type Browser, type Page, chromium, expect, test } from "@playwright/test";

import { catalogue, type Cle } from "../src/i18n/fr-ML";

const EXECUTABLE = join(import.meta.dirname, "..", "..", "target", "debug", "gesco-app.exe");
const CAPTURES = join(import.meta.dirname, "captures");
const IDENTIFIANT = "direction";
const MOT_DE_PASSE = "essai-premier-demarrage";
const NOUVEAU_MOT_DE_PASSE = "essai-mot-de-passe-change";
const RACINE_CARREE = "La racine carrée d'un nombre positif";
const LECON_WORD = "Cercle et disque, fiche de l'école";
/** Le titre écrit en tête de la fiche Word. */
const TITRE_FICHE = "Le cercle et le disque";

const texte = (cle: Cle, valeurs: Record<string, string | number> = {}) =>
  catalogue[cle].replace(/\{(\w+)\}/g, (entier, nom: string) => (valeurs[nom] === undefined ? entier : String(valeurs[nom])));

interface Instance {
  processus: ChildProcess;
  navigateur: Browser;
  page: Page;
}

let dossier: string;
let ecoleA: Instance;
let ecoleB: Instance | undefined;
let fichierLecons: string;

async function lancer(nom: string, portDebogage: number): Promise<Instance> {
  const donnees = join(dossier, nom, "donnees");
  // Le paquet signé de la bibliothèque, déposé comme le fait l'installeur.
  const paquets = process.env["GESCO_PAQUETS_BIBLIOTHEQUE"] ?? join(import.meta.dirname, "..", "..", "..", "..", "gesco-bibliotheque-paquets");
  mkdirSync(join(donnees, "bibliotheque"), { recursive: true });
  for (const fichier of readdirSync(paquets).filter((n) => n.endsWith(".gbib"))) {
    copyFileSync(join(paquets, fichier), join(donnees, "bibliotheque", fichier));
  }
  const processus = spawn(EXECUTABLE, [], {
    env: {
      ...process.env,
      GESCO_DOSSIER_ESSAI: donnees,
      GESCO_DATE_ESSAI: "2026-10-14",
      GESCO_HEURE_ESSAI: "10",
      WEBVIEW2_USER_DATA_FOLDER: join(dossier, nom, "webview"),
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${String(portDebogage)} --autoplay-policy=no-user-gesture-required`,
    },
    stdio: "ignore",
  });
  for (let essai = 0; essai < 60; essai += 1) {
    try {
      const navigateur = await chromium.connectOverCDP(`http://127.0.0.1:${String(portDebogage)}`);
      const [contexte] = navigateur.contexts();
      if (contexte === undefined) throw new Error("WebView2 n'expose aucun contexte");
      const page = contexte.pages()[0] ?? (await contexte.waitForEvent("page"));
      return { processus, navigateur, page };
    } catch {
      await new Promise((reprise) => setTimeout(reprise, 500));
    }
  }
  throw new Error(`WebView2 ne répond pas sur le port ${String(portDebogage)}`);
}

async function arreter(instance: Instance | undefined) {
  await instance?.navigateur.close().catch(() => undefined);
  if (instance?.processus.pid !== undefined) spawnSync("taskkill", ["/pid", String(instance.processus.pid), "/T", "/F"]);
}

const DEPLIER = "html, body, #racine, .h-screen { height: auto !important; } main { overflow: visible !important; }";

async function capturer(page: Page, nom: string) {
  const depli = await page.addStyleTag({ content: DEPLIER });
  await page.screenshot({ path: join(CAPTURES, `${nom}.png`), fullPage: true });
  await depli.evaluate((style) => style.parentNode?.removeChild(style));
}

async function allerA(page: Page, cle: Cle) {
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

/** La boîte « Ouvrir » du système rend ce chemin, au lieu de s'ouvrir. */
async function substituerOuverture(page: Page, chemin: string) {
  await substituer(page, "plugin:dialog|open", chemin);
}

/** La boîte « Enregistrer sous » du système rend ce chemin, au lieu de s'ouvrir. */
async function substituerEnregistrement(page: Page, chemin: string) {
  await substituer(page, "plugin:dialog|save", chemin);
}

async function substituer(page: Page, commande: string, chemin: string) {
  await page.evaluate(
    ([c, cmd]) => {
      const fenetre = window as unknown as { fetchOriginal?: typeof fetch };
      const original = fenetre.fetchOriginal ?? window.fetch.bind(window);
      fenetre.fetchOriginal = original;
      window.fetch = (entree, options) => {
        const url = decodeURIComponent(typeof entree === "string" ? entree : entree instanceof URL ? entree.href : entree.url);
        if (url.includes(`ipc.localhost/${cmd}`)) {
          return Promise.resolve(new Response(JSON.stringify(c), { status: 200, headers: { "Tauri-Response": "ok", "content-type": "application/json" } }));
        }
        return original(entree, options);
      };
    },
    [chemin, commande],
  );
}

async function rendreDialogues(page: Page) {
  await page.evaluate(() => {
    const fenetre = window as unknown as { fetchOriginal?: typeof fetch };
    if (fenetre.fetchOriginal !== undefined) window.fetch = fenetre.fetchOriginal;
    delete fenetre.fetchOriginal;
  });
}

/** Une école neuve (fondamental), sa directrice connectée, son mot de passe changé. */
async function creerEcole(page: Page, nom: string) {
  await expect(page.getByRole("heading", { name: texte("demarrage.titre") })).toBeVisible({ timeout: 30_000 });
  const champs: [Cle, string][] = [
    ["demarrage.organisation", nom],
    ["demarrage.etablissement_nom", nom],
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
  // La création installe la bibliothèque de cours déposée.
  await expect(page.getByRole("heading", { name: texte("phrase.titre") })).toBeVisible({ timeout: 90_000 });
  await page.getByLabel(texte("phrase.confirmation")).check();
  await page.getByRole("button", { name: texte("commun.continuer") }).click();
  await page.getByLabel(texte("connexion.identifiant")).fill(IDENTIFIANT);
  await page.getByLabel(texte("connexion.mot_de_passe")).fill(MOT_DE_PASSE);
  await page.getByRole("button", { name: texte("connexion.valider") }).click();
  await expect(page.getByText(texte("compte.changement_exige"))).toBeVisible();
  const motDePasse = page.locator("form").first();
  await motDePasse.getByLabel(texte("compte.mot_de_passe_actuel")).fill(MOT_DE_PASSE);
  await motDePasse.getByLabel(texte("compte.nouveau_mot_de_passe")).fill(NOUVEAU_MOT_DE_PASSE);
  await motDePasse.getByRole("button", { name: texte("commun.enregistrer") }).click();
  await expect(page.getByText(texte("compte.changement_exige"))).toBeHidden();
}

async function chercherEtOuvrir(page: Page, mots: string, titre: string | RegExp) {
  const champ = page.locator("#bibliotheque-recherche");
  await champ.fill("");
  await champ.fill(mots);
  const resultat = page.getByRole("button").filter({ hasText: titre }).first();
  await resultat.waitFor();
  await resultat.click();
}

/** Ouvre la matière d'un niveau dans l'arbre du catalogue. */
async function ouvrirMatiere(page: Page, niveau: RegExp, matiere: string) {
  const arbre = page.getByRole("navigation", { name: texte("bibliotheque.titre") });
  const blocNiveau = arbre.locator("details").filter({ has: page.locator(":scope > summary", { hasText: niveau }) }).first();
  const resume = blocNiveau.locator(":scope > summary");
  if ((await blocNiveau.getAttribute("open")) === null) await resume.click();
  const blocMatiere = blocNiveau.locator("details").filter({ has: page.locator(":scope > summary", { hasText: matiere }) }).first();
  if ((await blocMatiere.getAttribute("open")) === null) await blocMatiere.locator(":scope > summary").click();
  return blocMatiere;
}

/** La section « Leçons d'une autre école » des Paramètres. */
function sectionEchange(page: Page) {
  return page.locator("section").filter({ has: page.getByRole("heading", { name: texte("bibliotheque.echange.titre"), exact: true }) });
}

/** Capture de la seule section de l'échange : la page des Paramètres est trop longue pour se lire. */
async function capturerEchange(page: Page, nom: string) {
  await sectionEchange(page).screenshot({ path: join(CAPTURES, `${nom}.png`) });
}

test.describe.configure({ mode: "serial" });

test.beforeAll(async () => {
  dossier = mkdtempSync(join(tmpdir(), "gesco-lot2-"));
  ecoleA = await lancer("ecole-a", 9225);
});

test.afterAll(async () => {
  await arreter(ecoleB);
  await arreter(ecoleA);
  try {
    rmSync(dossier, { recursive: true, force: true, maxRetries: 10, retryDelay: 500 });
  } catch {
    return;
  }
});

test("lot 2 : l'école A est créée avec la bibliothèque livrée", async () => {
  test.setTimeout(240_000);
  await creerEcole(ecoleA.page, "École Askia");
});

test("lot 2 : un son s'ajoute à une leçon, se lit, et passe en mode classe ; un son trop lourd et un faux MP3 sont refusés", async () => {
  test.setTimeout(240_000);
  const page = ecoleA.page;
  await allerA(page, "nav.bibliotheque");
  await chercherEtOuvrir(page, "racine carree", RACINE_CARREE);
  await page.getByRole("button", { name: texte("bibliotheque.mode.ecrire"), exact: true }).click();
  await page.getByRole("button", { name: texte("bibliotheque.editeur.commencer"), exact: true }).click();
  const editeur = page.locator("#bibliotheque-editeur");
  await expect(editeur).not.toHaveValue("");
  await editeur.press("Control+End");
  await editeur.pressSequentially("\n\nÉcoutez la consigne :\n\n");

  // Un son trop lourd, puis un PNG renommé « .mp3 » : refusés, chacun avec son message.
  const lourd = join(dossier, "long.mp3");
  writeFileSync(lourd, Buffer.concat([Buffer.from("ID3\x03\0\0\0\0\0\0", "latin1"), Buffer.alloc(12 * 1024 * 1024, 1)]));
  await substituerOuverture(page, lourd);
  await page.getByRole("button", { name: texte("bibliotheque.editeur.son"), exact: true }).click();
  await expect(page.getByText(/Ce son dépasse 10 Mo/)).toBeVisible({ timeout: 30_000 });
  await capturer(page, "300-lot2-son-trop-lourd");
  await rendreDialogues(page);
  const faux = join(dossier, "faux.mp3");
  copyFileSync(join(import.meta.dirname, "logo-essai.png"), faux);
  await substituerOuverture(page, faux);
  await page.getByRole("button", { name: texte("bibliotheque.editeur.son"), exact: true }).click();
  await expect(page.getByText(/n'est pas un son/)).toBeVisible({ timeout: 30_000 });
  await rendreDialogues(page);

  // Un vrai son MP3 : appelé là où est le curseur, l'aperçu offre « Écouter ».
  const mp3 = join(dossier, "Consigne.mp3");
  copyFileSync(join(import.meta.dirname, "son-essai.mp3"), mp3);
  await substituerOuverture(page, mp3);
  await page.getByRole("button", { name: texte("bibliotheque.editeur.son"), exact: true }).click();
  await expect(editeur).toHaveValue(/son:[0-9a-f]{64}/, { timeout: 30_000 });
  await rendreDialogues(page);
  await expect(page.getByRole("button", { name: texte("bibliotheque.son.ecouter"), exact: true }).first()).toBeVisible({ timeout: 15_000 });
  await capturer(page, "301-lot2-son-editeur");

  await page.getByRole("button", { name: texte("bibliotheque.editeur.enregistrer"), exact: true }).click();
  await page.getByRole("button", { name: texte("bibliotheque.editeur.publier"), exact: true }).first().click();
  await expect(page.getByRole("button", { name: texte("bibliotheque.editeur.commencer"), exact: true })).toBeVisible();

  // À la lecture : le son se charge à la demande, puis le lecteur du système le joue.
  await page.getByRole("button", { name: texte("bibliotheque.mode.lire"), exact: true }).click();
  const lecon = page.locator("article");
  await lecon.getByRole("button", { name: texte("bibliotheque.son.ecouter"), exact: true }).click();
  const lecteur = lecon.locator("audio");
  await expect(lecteur).toBeVisible({ timeout: 30_000 });
  await expect.poll(() => lecteur.evaluate((a: HTMLAudioElement) => (Number.isFinite(a.duration) ? Math.round(a.duration) : 0)), { timeout: 15_000 }).toBe(3);
  await capturer(page, "302-lot2-son-lecture");

  // En mode classe : un grand bouton.
  await page.getByRole("button", { name: texte("bibliotheque.mode_classe"), exact: true }).click();
  const classe = page.getByRole("dialog", { name: RACINE_CARREE });
  await expect(classe.getByRole("button", { name: texte("bibliotheque.son.ecouter"), exact: true })).toBeVisible();
  await capturer(page, "303-lot2-son-mode-classe");
  await page.keyboard.press("Escape");
  await expect(classe).toHaveCount(0);
});

test("lot 2 : une leçon de l'école s'ajoute depuis une fiche Word ; la formule est à retaper ; un .doc est refusé", async () => {
  test.setTimeout(240_000);
  const page = ecoleA.page;
  await allerA(page, "nav.bibliotheque");
  await page.getByRole("button", { name: texte("bibliotheque.ajouter"), exact: true }).click();
  const ajout = page.getByRole("region", { name: texte("bibliotheque.ajouter") });
  // Le premier chapitre de mathématiques de 9e.
  const chapitres = ajout.locator("#ajout-chapitre option");
  const libelles = await chapitres.allTextContents();
  const chapitre = libelles.find((l) => /9e/.test(l) && /Mathématiques/.test(l));
  expect(chapitre).toBeDefined();
  await ajout.locator("#ajout-chapitre").selectOption({ label: chapitre ?? "" });
  await ajout.locator("#ajout-titre-contenu").fill(LECON_WORD);

  // Un ancien fichier Word est refusé avec la consigne de l'enregistrer en .docx.
  const doc = join(dossier, "ancienne.doc");
  writeFileSync(doc, Buffer.concat([Buffer.from([0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1]), Buffer.alloc(2048)]));
  await substituerOuverture(page, doc);
  await ajout.getByRole("button", { name: texte("bibliotheque.ajouter.depuis_word"), exact: true }).click();
  await expect(page.getByText(/Enregistrez ce fichier au format \.docx/)).toBeVisible({ timeout: 30_000 });
  await capturer(page, "304-lot2-word-doc-refuse");
  await rendreDialogues(page);

  // La fiche Word : la leçon ajoutée reprend sa fiche, le brouillon est ouvert.
  const docx = join(dossier, "Fiche cercle.docx");
  copyFileSync(join(import.meta.dirname, "fiche-essai.docx"), docx);
  await substituerOuverture(page, docx);
  await ajout.getByRole("button", { name: texte("bibliotheque.ajouter.depuis_word"), exact: true }).click();
  const editeur = page.locator("#bibliotheque-editeur");
  await expect(editeur).toHaveValue(/# Le cercle et le disque/, { timeout: 30_000 });
  await rendreDialogues(page);
  await expect(editeur).toHaveValue(/Formule à retaper ici/);
  // L'aperçu : les titres, la liste, le tableau, les deux images ; la note de l'auteur.
  await expect(page.getByText(texte("bibliotheque.word.formules", { nombre: 1 }))).toBeVisible();
  const apercu = page.locator(".min-h-64").first();
  await expect(apercu.getByRole("heading", { name: TITRE_FICHE })).toBeVisible();
  await expect(apercu.getByRole("table")).toBeVisible();
  await expect(apercu.locator("li").filter({ hasText: "le diamètre vaut deux rayons" })).toBeVisible();
  await expect(apercu.locator("img")).toHaveCount(2);
  await capturer(page, "305-lot2-word-brouillon");

  // L'import a déjà enregistré le brouillon : il se publie tel quel.
  await expect(page.getByRole("button", { name: texte("bibliotheque.editeur.enregistrer"), exact: true })).toBeDisabled();
  await page.getByRole("button", { name: texte("bibliotheque.editeur.publier"), exact: true }).first().click();
  await expect(page.getByRole("button", { name: texte("bibliotheque.editeur.commencer"), exact: true })).toBeVisible();
  // Le personnel qui écrit lit la note ; devant la classe, elle ne paraît pas.
  await page.getByRole("button", { name: texte("bibliotheque.mode.lire"), exact: true }).click();
  await expect(page.locator("article").getByRole("table")).toBeVisible();
  await capturer(page, "306-lot2-word-publiee");
  await page.getByRole("button", { name: texte("bibliotheque.mode_classe"), exact: true }).click();
  const classe = page.getByRole("dialog", { name: LECON_WORD, exact: true });
  await expect(classe.getByRole("table")).toBeVisible();
  await expect(classe.getByText("Formule à retaper ici")).toHaveCount(0);
  await page.keyboard.press("Escape");
});

test("lot 2 : l'école A exporte les mathématiques de 9e en un fichier de leçons", async () => {
  const page = ecoleA.page;
  await allerA(page, "nav.bibliotheque");
  const matiere = await ouvrirMatiere(page, /^9e/, "Mathématiques");
  await matiere.getByRole("button", { name: texte("bibliotheque.echange.exporter_matiere"), exact: true }).first().click();
  await expect(page.getByText(texte("bibliotheque.echange.export_liste", { nombre: 2 }))).toBeVisible({ timeout: 30_000 });
  await capturer(page, "307-lot2-export-liste");
  fichierLecons = join(dossier, "lecons-askia.gles");
  await substituerEnregistrement(page, fichierLecons);
  await page.getByRole("button", { name: texte("bibliotheque.echange.enregistrer"), exact: true }).click();
  await expect(page.getByRole("status").filter({ hasText: texte("bibliotheque.echange.exporte", { nombre: 2 }) })).toBeVisible({ timeout: 60_000 });
  await rendreDialogues(page);
  expect(existsSync(fichierLecons)).toBe(true);
  expect(statSync(fichierLecons).size).toBeGreaterThan(1000);
  await capturer(page, "308-lot2-export-fait");
});

test("lot 2 : l'école B importe le fichier en brouillons ; le même fichier une seconde fois n'ajoute rien", async () => {
  test.setTimeout(300_000);
  await arreter(ecoleA);
  ecoleB = await lancer("ecole-b", 9226);
  const page = ecoleB.page;
  await creerEcole(page, "École Mandé");
  await allerA(page, "nav.parametres");
  await expect(sectionEchange(page)).toBeVisible();
  await substituerOuverture(page, fichierLecons);
  await page.getByRole("button", { name: texte("bibliotheque.echange.importer"), exact: true }).click();
  await expect(page.getByText(/lecons-askia\.gles : 2 leçon/)).toBeVisible({ timeout: 30_000 });
  await rendreDialogues(page);
  await expect(page.getByText(LECON_WORD).first()).toBeVisible();
  await expect(page.getByText(RACINE_CARREE).first()).toBeVisible();
  await capturerEchange(page, "309-lot2-import-apercu");
  await page.getByRole("button", { name: texte("bibliotheque.echange.confirmer", { nombre: 2 }), exact: true }).click();
  await expect(page.getByText(texte("bibliotheque.echange.resume.brouillons", { nombre: 2 }))).toBeVisible({ timeout: 60_000 });
  await capturerEchange(page, "310-lot2-import-fait");

  // Le même fichier, une seconde fois : l'aperçu dit qu'il est déjà venu, rien n'est ajouté.
  await substituerOuverture(page, fichierLecons);
  await page.getByRole("button", { name: texte("bibliotheque.echange.importer"), exact: true }).click();
  await expect(page.getByText(/a déjà été importé le/)).toBeVisible({ timeout: 30_000 });
  await rendreDialogues(page);
  const confirmer = page.getByRole("button", { name: /^Recevoir/ });
  if ((await confirmer.count()) > 0 && (await confirmer.isEnabled())) {
    await confirmer.click();
    await expect(page.getByText(texte("bibliotheque.echange.resume.rien"))).toBeVisible({ timeout: 60_000 });
  }
  await capturerEchange(page, "311-lot2-import-deja-la");
});

test("lot 2 : rien n'est visible avant publication ; l'école B publie l'import et lit le son reçu", async () => {
  test.setTimeout(240_000);
  if (ecoleB === undefined) throw new Error("école B absente");
  const page = ecoleB.page;
  // Avant publication, la leçon livrée se lit dans sa version livrée.
  await allerA(page, "nav.bibliotheque");
  await chercherEtOuvrir(page, "racine carree", RACINE_CARREE);
  await expect(page.locator("article").getByText("Écoutez la consigne")).toHaveCount(0);

  await allerA(page, "nav.parametres");
  await page.getByRole("button", { name: texte("bibliotheque.echange.publier_import", { nombre: 2 }), exact: true }).click();
  await page.getByRole("button", { name: texte("bibliotheque.echange.publier"), exact: true }).click();
  await expect(page.getByText(texte("bibliotheque.echange.publiees", { nombre: 2 }))).toBeVisible({ timeout: 60_000 });
  await capturerEchange(page, "312-lot2-import-publie");

  await allerA(page, "nav.bibliotheque");
  await chercherEtOuvrir(page, "racine carree", RACINE_CARREE);
  const lecon = page.locator("article");
  await expect(lecon.getByText("Écoutez la consigne")).toBeVisible();
  await lecon.getByRole("button", { name: texte("bibliotheque.son.ecouter"), exact: true }).click();
  await expect(lecon.locator("audio")).toBeVisible({ timeout: 30_000 });
  await capturer(page, "313-lot2-son-recu");
  await chercherEtOuvrir(page, "cercle disque fiche ecole", LECON_WORD);
  await expect(page.getByRole("heading", { name: LECON_WORD, exact: true })).toBeVisible();
  await expect(page.locator("article").getByRole("table")).toBeVisible();
  await expect(page.locator("article img")).toHaveCount(2);
  await capturer(page, "314-lot2-lecon-word-recue");
});

test("lot 2 : module éteint, ni la bibliothèque ni l'import de leçons ne paraissent ; rallumé, tout revient", async () => {
  if (ecoleB === undefined) throw new Error("école B absente");
  const page = ecoleB.page;
  await allerA(page, "nav.parametres");
  await page.locator("#module-bibliotheque").uncheck();
  await expect(page.getByRole("button", { name: texte("bibliotheque.echange.importer"), exact: true })).toHaveCount(0);
  await expect(page.getByRole("navigation").first().getByRole("button", { name: texte("nav.bibliotheque"), exact: true })).toHaveCount(0);
  await capturer(page, "315-lot2-module-eteint");
  await page.locator("#module-bibliotheque").check();
  await expect(page.getByRole("button", { name: texte("bibliotheque.echange.importer"), exact: true })).toBeVisible();
  await expect(page.getByRole("navigation").first().getByRole("button", { name: texte("nav.bibliotheque"), exact: true })).toHaveCount(1);
});
