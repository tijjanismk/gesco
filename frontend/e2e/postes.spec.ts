// Postes de l'école, sur la vraie application et sur deux instances de GESCO : le poste principal
// allume le réseau et affiche un code ; un second GESCO, au premier lancement, rejoint le poste
// principal, ouvre une session, exporte un classeur sur son propre disque, garde sa saisie quand
// le poste principal s'éteint et reprend seul quand il revient ; puis il est révoqué et quitte le
// poste principal. En mode d'essai, le serveur n'écoute que la machine elle-même.
import { type ChildProcess, spawn, spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { connect } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { type Browser, type Page, chromium, expect, test } from "@playwright/test";

import { catalogue, type Cle } from "../src/i18n/fr-ML";

const EXECUTABLE = join(import.meta.dirname, "..", "..", "target", "debug", "gesco-app.exe");
const CAPTURES = join(import.meta.dirname, "captures");
const PORT_RESEAU = 47800;
const IDENTIFIANT = "direction";
const NOM = "Awa Traoré";
const MOT_DE_PASSE = "essai-premier-demarrage";
const NOUVEAU_MOT_DE_PASSE = "essai-mot-de-passe-change";
const POSTE = "Caisse";
const MAITRE = "moussa";
const NOM_MAITRE = "Moussa Diarra";
const MOT_DE_PASSE_MAITRE = "maitre-2026-initial";
const EMPREINTE = /^[0-9A-F]{4}(-[0-9A-F]{4}){3}$/;

const texte = (cle: Cle, valeurs: Record<string, string> = {}) =>
  catalogue[cle].replace(/\{(\w+)\}/g, (entier, nom: string) => valeurs[nom] ?? entier);

/** Une instance de GESCO : son processus, sa fenêtre et son espace de données. */
interface Instance {
  processus: ChildProcess;
  navigateur: Browser;
  page: Page;
  donnees: string;
}

let dossier: string;
let principal: Instance;
let relie: Instance;

async function lancer(nom: string, portDebogage: number): Promise<Instance> {
  const donnees = join(dossier, nom, "donnees");
  const processus = spawn(EXECUTABLE, [], {
    env: {
      ...process.env,
      GESCO_DOSSIER_ESSAI: donnees,
      GESCO_DATE_ESSAI: "2026-10-14",
      GESCO_HEURE_ESSAI: "10",
      WEBVIEW2_USER_DATA_FOLDER: join(dossier, nom, "webview"),
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${String(portDebogage)}`,
    },
    stdio: "ignore",
  });
  for (let essai = 0; essai < 60; essai += 1) {
    try {
      const navigateur = await chromium.connectOverCDP(`http://127.0.0.1:${String(portDebogage)}`);
      const [contexte] = navigateur.contexts();
      if (contexte === undefined) throw new Error("WebView2 n'expose aucun contexte");
      const page = contexte.pages()[0] ?? (await contexte.waitForEvent("page"));
      return { processus, navigateur, page, donnees };
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
  const navigation = page.getByRole("navigation");
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

/** Vrai si un programme écoute le port du réseau de l'école sur cette machine. */
function portOuvert(): Promise<boolean> {
  return new Promise((resoudre) => {
    const prise = connect({ host: "127.0.0.1", port: PORT_RESEAU });
    prise.once("connect", () => {
      prise.destroy();
      resoudre(true);
    });
    prise.once("error", () => resoudre(false));
  });
}

async function connecter(page: Page, motDePasse: string) {
  await page.getByLabel(texte("connexion.identifiant")).fill(IDENTIFIANT);
  await page.getByLabel(texte("connexion.mot_de_passe")).fill(motDePasse);
  await page.getByRole("button", { name: texte("connexion.valider") }).click();
}

test.describe.configure({ mode: "serial" });

test.beforeAll(async () => {
  dossier = mkdtempSync(join(tmpdir(), "gesco-postes-"));
  principal = await lancer("principal", 9223);
});

test.afterAll(async () => {
  await arreter(relie);
  await arreter(principal);
  try {
    rmSync(dossier, { recursive: true, force: true, maxRetries: 10, retryDelay: 500 });
  } catch {
    return;
  }
});

test("une école d'essai et sa directrice", async () => {
  const page = principal.page;
  await expect(page.getByRole("heading", { name: texte("demarrage.titre") })).toBeVisible({ timeout: 30_000 });
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
  await page.locator("#cycle-premier_cycle").check();
  await page.locator("form").getByRole("button", { name: texte("demarrage.creer") }).click();
  await expect(page.getByRole("heading", { name: texte("phrase.titre") })).toBeVisible({ timeout: 30_000 });
  await page.getByLabel(texte("phrase.confirmation")).check();
  await page.getByRole("button", { name: texte("commun.continuer") }).click();

  await connecter(page, MOT_DE_PASSE);
  await expect(page.getByText(texte("compte.changement_exige"))).toBeVisible();
  const motDePasse = page.locator("form").first();
  await motDePasse.getByLabel(texte("compte.mot_de_passe_actuel")).fill(MOT_DE_PASSE);
  await motDePasse.getByLabel(texte("compte.nouveau_mot_de_passe")).fill(NOUVEAU_MOT_DE_PASSE);
  await motDePasse.getByRole("button", { name: texte("commun.enregistrer") }).click();
  await expect(page.getByText(texte("compte.changement_exige"))).toBeHidden();
});

test("la direction allume le réseau de l'école", async () => {
  const page = principal.page;
  await allerA(page, "nav.postes");
  await expect(page.getByRole("heading", { name: texte("postes.titre"), level: 1 })).toBeVisible();
  await expect(page.getByText(texte("postes.reseau.eteint"))).toBeVisible();
  await expect(page.getByText(texte("postes.liste_vide"))).toBeVisible();
  await capturer(page, "140-postes-reseau-eteint");

  // Tant que le réseau est éteint, personne ne répond sur le port.
  expect(await portOuvert()).toBe(false);

  // Activer ouvre d'abord le pare-feu de Windows. Le pare-feu des essais refuse la première
  // demande, comme une personne qui répond « Non » à Windows : le réseau s'allume, l'écran dit
  // que les autres ordinateurs n'entreront pas et offre de recommencer.
  await expect(page.getByText(texte("postes.activer_aide"))).toBeVisible();
  await page.getByRole("button", { name: texte("postes.activer") }).click();
  await expect(page.getByText(texte("postes.reseau.allume"))).toBeVisible();
  await expect(page.getByText(texte("postes.pare_feu.ferme"))).toBeVisible();
  await capturer(page, "162-postes-pare-feu-ferme");
  // Un réseau que Windows range parmi les réseaux publics : l'écran dit quoi régler.
  const reseauPublic = join(principal.donnees, "essai-reseau-public");
  writeFileSync(reseauPublic, "");
  await page.getByRole("button", { name: texte("postes.pare_feu.ouvrir") }).click();
  await expect(page.getByText(texte("postes.pare_feu.reseau_public"))).toBeVisible();
  await expect(page.getByText(texte("postes.pare_feu.ouvert"))).toBeVisible();
  await capturer(page, "163-postes-reseau-public");
  // Le réseau déclaré privé dans Windows : l'écran le constate à la demande.
  rmSync(reseauPublic);
  await page.getByRole("button", { name: texte("postes.pare_feu.verifier") }).click();
  await expect(page.getByText(texte("postes.pare_feu.reseau_public"))).toBeHidden();
  await expect(page.getByText(texte("postes.pare_feu.ouvert"))).toBeVisible();
  await expect(page.getByText(texte("postes.pare_feu.ferme"))).toBeHidden();
  // Le serveur fabrique le certificat à la première activation : son empreinte paraît à la
  // relecture suivante de l'écran.
  await expect(page.getByText(texte("postes.empreinte_attente"))).toBeHidden({ timeout: 20_000 });
  await expect(page.getByText(EMPREINTE)).toBeVisible();
  await expect(page.getByText("127.0.0.1", { exact: true })).toBeVisible();
  await capturer(page, "141-postes-reseau-allume");
});

test("un second GESCO rejoint le poste principal par le code affiché", async () => {
  relie = await lancer("relie", 9224);
  const page = relie.page;
  // Premier lancement : trois chemins, dont le troisième.
  await expect(page.getByRole("heading", { name: texte("demarrage.titre") })).toBeVisible({ timeout: 30_000 });
  await page.getByRole("button", { name: texte("demarrage.rejoindre") }).click();
  await expect(page.getByRole("heading", { name: texte("rejoindre.titre") })).toBeVisible();

  // Une adresse où personne ne répond : le poste le dit, sans rien relier.
  await page.getByLabel(texte("rejoindre.adresse")).fill("127.0.0.1:47999");
  await page.getByRole("button", { name: texte("rejoindre.chercher") }).click();
  await expect(page.getByRole("alert")).toContainText(texte("reseau.poste_principal_injoignable").slice(0, 40), { timeout: 20_000 });

  // La bonne adresse : l'empreinte lue par ce poste est celle que montre le poste principal.
  await page.getByLabel(texte("rejoindre.adresse")).fill("127.0.0.1");
  await page.getByRole("button", { name: texte("rejoindre.chercher") }).click();
  const empreinteVue = page.locator("#rejoindre-empreinte");
  await expect(empreinteVue).toHaveText(EMPREINTE, { timeout: 20_000 });
  const empreinteAffichee = (await principal.page.getByText(EMPREINTE).innerText()).trim();
  await expect(empreinteVue).toHaveText(empreinteAffichee);

  await principal.page.getByRole("button", { name: texte("postes.code_demander") }).click();
  const zoneCode = principal.page.getByRole("status");
  await expect(zoneCode).toHaveText(/^[A-Z2-9]{4} [A-Z2-9]{4}$/);
  const code = (await zoneCode.innerText()).trim();
  await capturer(principal.page, "142-postes-code-a-saisir");

  // Le nom de l'ordinateur est proposé ; un code faux ne relie rien.
  await expect(page.getByLabel(texte("rejoindre.nom"))).not.toHaveValue("");
  await page.getByLabel(texte("rejoindre.nom")).fill(POSTE);
  await page.getByLabel(texte("rejoindre.code")).fill("ZZZZZZZZ");
  await page.getByRole("button", { name: texte("rejoindre.relier") }).click();
  await expect(page.getByRole("alert")).toHaveText(texte("reseau.code_faux"));
  // Le bon code, saisi en minuscules avec son espace, relie le poste.
  await page.getByLabel(texte("rejoindre.code")).fill(code.toLowerCase());
  await capturer(page, "147-rejoindre-le-poste-principal");
  await page.getByRole("button", { name: texte("rejoindre.relier") }).click();

  // Le poste est relié : écran de connexion de l'école, sous le bandeau du poste.
  await expect(page.getByText(texte("bandeau_poste.nom", { nom: POSTE }))).toBeVisible({ timeout: 20_000 });
  await expect(page.getByText(texte("bandeau_poste.etablie"))).toBeVisible();
  await expect(page.getByText("École fondamentale Askia")).toBeVisible();
  await capturer(page, "148-poste-relie-connexion");
  // Il ne garde ni base ni clé de données ; son réglage dit son rôle, hors de toute base.
  expect(existsSync(join(relie.donnees, "gesco.db"))).toBe(false);
  expect(existsSync(join(relie.donnees, "cles.json"))).toBe(false);
  expect(JSON.parse(readFileSync(join(relie.donnees, "poste.json"), "utf8"))).toEqual({ role: "relie", nom: POSTE });
});

test("le poste relié travaille sous son nom, sans les écrans du poste principal", async () => {
  const page = relie.page;
  await connecter(page, NOUVEAU_MOT_DE_PASSE);
  const navigation = page.getByRole("navigation");
  await expect(navigation.getByRole("button", { name: texte("nav.accueil"), exact: true })).toBeVisible({ timeout: 20_000 });
  await expect(page.getByText(texte("bandeau.utilisateur", { nom: NOM }))).toBeVisible();
  // Aucune année n'est encore ouverte : le poste relié le dit, comme le poste principal.
  await expect(page.getByText(texte("avertissement.annee_absente"))).toBeVisible();

  // Ce qui dépend du disque du poste principal n'y paraît pas : sauvegardes, postes de l'école.
  await allerA(page, "nav.utilisateurs");
  await expect(navigation.getByRole("button", { name: texte("nav.journal"), exact: true })).toBeVisible();
  await expect(navigation.getByRole("button", { name: texte("nav.sauvegarde"), exact: true })).toHaveCount(0);
  await expect(navigation.getByRole("button", { name: texte("nav.postes"), exact: true })).toHaveCount(0);

  // Un export Excel : le classeur, composé sur le poste principal, s'écrit sur le disque du
  // poste relié. Seul l'appel du dialogue d'enregistrement est substitué.
  const classeur = join(relie.donnees, "utilisateurs.xlsx");
  await page.evaluate((chemin) => {
    const original = window.fetch.bind(window);
    window.fetch = (entree, options) => {
      const url = typeof entree === "string" ? entree : entree instanceof URL ? entree.href : entree.url;
      if (decodeURIComponent(url).includes("ipc.localhost/plugin:dialog|save")) {
        return Promise.resolve(new Response(JSON.stringify(chemin), { status: 200, headers: { "Tauri-Response": "ok", "content-type": "application/json" } }));
      }
      return original(entree, options);
    };
  }, classeur);
  await expect(page.getByRole("cell", { name: NOM }).first()).toBeVisible();
  await page.getByRole("button", { name: texte("commun.exporter_excel") }).click();
  await expect(page.getByText(texte("commun.export_enregistre", { chemin: classeur }))).toBeVisible({ timeout: 20_000 });
  expect(readFileSync(classeur).subarray(0, 2).toString("latin1")).toBe("PK");
  // Rien ne reste sur le poste principal.
  expect(existsSync(join(principal.donnees, "utilisateurs.xlsx"))).toBe(false);
  await capturer(page, "149-poste-relie-au-travail");

  // Le poste principal le voit connecté ; sans licence, un seul poste : la place est prise.
  const ligne = principal.page.getByRole("row", { name: new RegExp(`^${POSTE}`) });
  await expect(ligne).toBeVisible({ timeout: 20_000 });
  await expect(ligne.getByText(texte("postes.etat.connecte"))).toBeVisible({ timeout: 20_000 });
  await expect(principal.page.getByText(texte("postes.complet"))).toBeVisible();
  await expect(principal.page.getByRole("button", { name: texte("postes.code_demander") })).toBeDisabled();
  await capturer(principal.page, "143-postes-poste-relie");

  // Le journal, lu depuis le poste relié, porte son nom sur ses écritures.
  await allerA(page, "nav.journal");
  await expect(page.getByRole("heading", { name: texte("journal.titre") })).toBeVisible();
  await expect(page.getByRole("cell", { name: POSTE, exact: true }).first()).toBeVisible({ timeout: 20_000 });
});

test("fermer le poste principal quand un poste travaille demande confirmation", async () => {
  // Demande de fermeture de la fenêtre, comme un clic sur sa croix.
  if (principal.processus.pid !== undefined) spawnSync("taskkill", ["/pid", String(principal.processus.pid)]);
  const avertissement = principal.page.getByRole("alertdialog", { name: texte("fermeture.titre") });
  await expect(avertissement).toBeVisible({ timeout: 20_000 });
  await expect(avertissement).toContainText(texte("fermeture.un_poste"));
  await capturer(principal.page, "150-fermeture-du-poste-principal");
  await avertissement.getByRole("button", { name: texte("fermeture.rester") }).click();
  await expect(avertissement).toBeHidden();
  // GESCO est resté ouvert et sert toujours le poste relié.
  expect(await portOuvert()).toBe(true);
});

test("la même fiche modifiée de deux postes : la seconde est refusée et garde sa saisie", async () => {
  const adresse = (page: Page) => page.locator("#etablissement-adresse");
  const telephones = (page: Page) => page.locator("#etablissement-telephones");
  const fiche = (page: Page) => page.locator("form").filter({ has: adresse(page) });
  const enregistrer = (page: Page) => fiche(page).getByRole("button", { name: texte("commun.enregistrer") }).click();
  const enregistree = (page: Page) => expect(fiche(page).getByRole("status")).toContainText(texte("commun.enregistre_a", { heure: "" }), { timeout: 20_000 });

  // Les deux postes ont la fiche de l'établissement sous les yeux.
  for (const page of [principal.page, relie.page]) {
    await allerA(page, "nav.parametres");
    await expect(adresse(page)).toHaveValue("Hamdallaye ACI 2000, Bamako", { timeout: 20_000 });
  }
  // Le poste principal corrige l'adresse.
  await adresse(principal.page).fill("Badalabougou, Bamako");
  await enregistrer(principal.page);
  await enregistree(principal.page);

  // Le poste relié, sur la fiche d'avant, corrige le téléphone : rien ne s'enregistre, le refus
  // dit qui a modifié la fiche et à quelle heure, et la saisie reste à l'écran.
  const page = relie.page;
  await telephones(page).fill("+223 20 22 11 11");
  await enregistrer(page);
  const refus = fiche(page).getByRole("alert");
  await expect(refus).toContainText(texte("fiche.modifiee_entre_temps", { personne: NOM, heure: "" }).split(NOM)[0] + NOM, { timeout: 20_000 });
  await expect(refus).toContainText(/à \d{1,2} h \d{2}|à \d{1,2}:\d{2}/);
  await expect(telephones(page)).toHaveValue("+223 20 22 11 11");
  await expect(adresse(page)).toHaveValue("Hamdallaye ACI 2000, Bamako");
  await capturer(page, "153-fiche-modifiee-entre-temps");

  // Relire la fiche montre la correction du collègue ; la sienne se refait et s'enregistre.
  await refus.getByRole("button", { name: texte("fiche.relire") }).click();
  await expect(adresse(page)).toHaveValue("Badalabougou, Bamako", { timeout: 20_000 });
  await expect(refus).toBeHidden();
  await telephones(page).fill("+223 20 22 11 11");
  await enregistrer(page);
  await enregistree(page);
  await capturer(page, "154-fiche-relue-puis-enregistree");

  // Écrans qui se relisent : en revenant sur l'écran, le poste principal lit les deux corrections.
  await allerA(principal.page, "nav.utilisateurs");
  await allerA(principal.page, "nav.parametres");
  await expect(telephones(principal.page)).toHaveValue("+223 20 22 11 11", { timeout: 20_000 });
  await expect(adresse(principal.page)).toHaveValue("Badalabougou, Bamako");
});

test("avant une opération qui demande d'être seul, le poste principal montre les postes connectés", async () => {
  const page = principal.page;
  await allerA(page, "nav.annees");
  await page.locator("#nouvelle-annee-libelle").fill("2026-2027");
  await page.locator("#nouvelle-annee-debut").fill("2026-10-01");
  await page.locator("#nouvelle-annee-fin").fill("2027-07-31");
  await page.getByRole("button", { name: texte("annees.creer") }).click();
  await page.getByRole("cell", { name: "2026-2027" }).click();
  await page.getByRole("button", { name: texte("annees.ouvrir") }).click();
  await expect(page.getByText(texte("bandeau.annee").replace("{annee}", "2026-2027"), { exact: false })).toBeVisible();

  // Clôturer l'année : le poste relié a une session ouverte, la confirmation le nomme.
  await allerA(page, "nav.fin_annee");
  await page.getByRole("button", { name: texte("fin_annee.cloturer", { annee: "2026-2027" }) }).click();
  await page.getByRole("button", { name: texte("fin_annee.confirmer_cloture", { annee: "2026-2027" }) }).click();
  const confirmation = page.getByRole("alertdialog", { name: texte("fermeture.titre") });
  await expect(confirmation).toBeVisible({ timeout: 20_000 });
  await expect(confirmation.getByRole("listitem")).toHaveText([POSTE]);
  await expect(confirmation).toContainText(texte("seul.cloture"));
  await capturer(page, "155-operation-seule-postes-connectes");
  // La direction renonce : rien n'est clôturé.
  await confirmation.getByRole("button", { name: texte("commun.annuler") }).click();
  await expect(confirmation).toBeHidden();
  await expect(page.getByRole("button", { name: texte("fin_annee.confirmer_cloture", { annee: "2026-2027" }) })).toBeVisible();
  await allerA(page, "nav.postes");
});

test("le poste relié inscrit un élève, encaisse son premier versement et imprime le reçu", async () => {
  // Le poste principal prépare l'année : une classe et ses frais, mis en vigueur.
  const direction = principal.page;
  await allerA(direction, "nav.classes");
  await direction.locator("#classe-niveau").selectOption({ label: "3e année" });
  await direction.locator("#classe-libelle").fill("3e A");
  await direction.getByRole("button", { name: texte("classes.creer") }).click();
  await expect(direction.getByRole("cell", { name: "3e A", exact: true })).toBeVisible({ timeout: 20_000 });
  await allerA(direction, "nav.frais");
  await direction.getByRole("button", { name: texte("frais.nouvelle") }).click();
  await direction.getByRole("button", { name: texte("frais.ajouter_ligne") }).click();
  await direction.locator("#frais-ligne-0-niveau-3").check();
  await direction.locator("#frais-ligne-0-montant").fill("150000");
  await direction.getByRole("button", { name: texte("commun.enregistrer") }).click();
  await expect(direction.getByText(/Enregistré à/)).toBeVisible({ timeout: 20_000 });
  await direction.getByRole("button", { name: texte("frais.mettre_en_vigueur") }).click();
  await expect(direction.getByRole("heading", { name: `Frais 2026-2027 · ${texte("frais.etat.en_vigueur")}` })).toBeVisible({ timeout: 20_000 });

  // Le poste relié ouvre sa caisse.
  const page = relie.page;
  await allerA(page, "nav.caisse");
  // Le poste principal a ouvert l'année : au premier écran ouvert, l'avertissement du poste
  // relié s'en va, sans qu'il ait à se reconnecter.
  await expect(page.getByText(texte("avertissement.annee_absente"))).toHaveCount(0);
  await page.locator("#caisse-fonds").fill("25000");
  await page.getByRole("button", { name: texte("caisse.ouvrir_bouton") }).click();
  await expect(page.getByRole("heading", { name: /Caisse ouverte depuis le/ })).toBeVisible({ timeout: 20_000 });

  // L'assistant d'inscription, de la fiche de l'élève au reçu, sans quitter le poste relié.
  await allerA(page, "nav.inscriptions");
  await page.getByRole("button", { name: texte("inscriptions.assistant") }).click();
  await page.locator("#nouvel-eleve-nom").fill("Diallo");
  await page.locator("#nouvel-eleve-prenoms").fill("Fatoumata");
  await page.locator("#nouvel-eleve-sexe").selectOption("feminin");
  await page.locator("#nouvel-eleve-naissance").fill("2018-06-03");
  await page.locator("#nouvel-eleve-lieu_naissance").fill("Kati");
  await page.getByRole("button", { name: texte("eleves.creer") }).click();
  await page.locator("#ajout-lien").selectOption("mere");
  await page.locator("#nouveau-responsable-nom").fill("Diallo");
  await page.locator("#nouveau-responsable-prenoms").fill("Kadiatou");
  await page.locator("#nouveau-responsable-telephone-0").fill("76 45 12 90");
  await page.getByRole("button", { name: texte("responsables.creer_et_lier") }).click();
  await expect(page.getByText("+223 76 45 12 90")).toBeVisible({ timeout: 20_000 });
  await page.getByRole("button", { name: texte("commun.continuer") }).click();
  await page.locator("#inscription-classe").selectOption({ label: "3e A · 3e année" });
  await expect(page.locator("#inscription-plan")).toBeVisible();
  await page.getByRole("button", { name: texte("inscriptions.inscrire_et_valider") }).click();
  await expect(page.getByText(`2026-2027 · 3e A · ${texte("inscriptions.statut.validee")}`)).toBeVisible({ timeout: 20_000 });
  await page.getByRole("button", { name: texte("commun.continuer") }).click();

  // La facture, puis le premier versement : le reçu, composé sur le poste principal, s'affiche
  // sur le poste relié.
  await expect(page.getByText(/facture FAC-2026-/)).toBeVisible({ timeout: 20_000 });
  await page.locator("#encaissement-montant").fill("10000");
  await expect(page.locator("#encaissement-payeur")).toHaveValue(/.+/);
  await page.locator("#encaissement-montant").press("Enter");
  await expect(page.getByRole("status").filter({ hasText: /REC-2026-\d{6}/ })).toBeVisible({ timeout: 20_000 });
  await expect(page.getByText(/Reçu REC-2026-\d{6}/)).toBeVisible();
  await expect(page.locator("iframe").last()).toBeVisible({ timeout: 30_000 });
  await page.waitForTimeout(4000);
  await page.getByRole("heading", { name: texte("inscriptions.premier_versement") }).scrollIntoViewIfNeeded();
  await capturer(page, "156-poste-relie-inscription-versement-recu");

  // L'écriture porte le nom du poste relié, et le poste principal voit l'élève inscrit.
  await allerA(direction, "nav.inscriptions");
  await expect(direction.getByRole("cell", { name: /Diallo/ }).first()).toBeVisible({ timeout: 20_000 });
  await allerA(direction, "nav.postes");
});

test("saisie des notes par les maîtres : éteinte, le compte n'entre pas ; allumée, le maître ne voit que sa classe", async () => {
  // À la livraison, le réglage est éteint : la direction saisit tout.
  const direction = principal.page;
  await allerA(direction, "nav.parametres");
  const reglage = direction.locator('[id="param-notes.saisie_maitres"]');
  const ligne = reglage.locator("xpath=ancestor::div[contains(@class,'py-2')][1]");
  await expect(reglage).not.toBeChecked();
  await ligne.scrollIntoViewIfNeeded();
  await capturer(direction, "157-parametres-saisie-des-maitres-eteinte");

  // Le compte du maître, sa fiche d'enseignant et sa classe.
  await allerA(direction, "nav.utilisateurs");
  await direction.locator("#nouvel-identifiant").fill(MAITRE);
  await direction.locator("#nouveau-nom").fill(NOM_MAITRE);
  await direction.locator("#nouveau-mdp-initial").fill(MOT_DE_PASSE_MAITRE);
  await direction.locator("#nouveau-role-enseignant").check();
  await direction.getByRole("button", { name: texte("utilisateurs.creer") }).click();
  await expect(direction.getByRole("cell", { name: NOM_MAITRE })).toBeVisible({ timeout: 20_000 });
  await allerA(direction, "nav.enseignants");
  await direction.locator("#enseignant-prenoms").fill("Moussa");
  await direction.locator("#enseignant-nom").fill("Diarra");
  await direction.locator("#enseignant-telephone").fill("76 11 22 33");
  await direction.locator("#enseignant-statut").selectOption("titulaire");
  await direction.locator("#enseignant-compte").selectOption({ label: NOM_MAITRE });
  await direction.getByRole("button", { name: texte("commun.enregistrer") }).click();
  await expect(direction.getByRole("cell", { name: "Moussa", exact: true })).toBeVisible({ timeout: 20_000 });
  await direction.locator("#affectations-classe").selectOption({ label: "3e A · 3e année" });
  await direction.locator("#affectations-maitre").selectOption({ label: NOM_MAITRE });
  await direction.getByRole("button", { name: texte("enseignants.toute_la_classe") }).click();
  await expect(direction.getByRole("status").filter({ hasText: /matières confiées/ })).toBeVisible({ timeout: 20_000 });
  // Une élève qui n'est pas de sa classe : sa fiche existe, sans inscription.
  await allerA(direction, "nav.eleves");
  await direction.getByRole("button", { name: texte("eleves.nouveau") }).click();
  await direction.locator("#nouvel-eleve-nom").fill("Keita");
  await direction.locator("#nouvel-eleve-prenoms").fill("Aïssata");
  await direction.locator("#nouvel-eleve-sexe").selectOption("feminin");
  await direction.locator("#nouvel-eleve-naissance").fill("2017-02-11");
  await direction.locator("#nouvel-eleve-lieu_naissance").fill("Bamako");
  await direction.getByRole("button", { name: texte("eleves.creer") }).click();
  await expect(direction.getByRole("heading", { name: "Aïssata Keita" })).toBeVisible({ timeout: 20_000 });
  // La direction, elle, lit la situation financière sur la fiche d'une élève.
  await direction.locator("#eleves-recherche").fill("Fatoumata");
  await direction.getByRole("cell", { name: "Fatoumata", exact: true }).click();
  await expect(direction.getByRole("heading", { name: "Fatoumata Diallo" })).toBeVisible({ timeout: 20_000 });
  await expect(direction.getByRole("heading", { name: texte("facture.titre"), exact: true })).toBeVisible({ timeout: 20_000 });

  // Sur le poste relié, le maître donne son mot de passe : il n'entre pas, le message renvoie à
  // la direction.
  const page = relie.page;
  await page.getByRole("button", { name: texte("bandeau.deconnexion") }).click();
  await page.getByLabel(texte("connexion.identifiant")).fill(MAITRE);
  await page.getByLabel(texte("connexion.mot_de_passe")).fill(MOT_DE_PASSE_MAITRE);
  await page.getByRole("button", { name: texte("connexion.valider") }).click();
  await expect(page.getByRole("alert")).toContainText(texte("connexion.maitres_non_admis"), { timeout: 20_000 });
  await capturer(page, "158-maitre-refuse-saisie-eteinte");

  // La direction allume le réglage.
  await allerA(direction, "nav.parametres");
  await reglage.check();
  await ligne.getByRole("button", { name: texte("commun.enregistrer") }).click();
  await expect(ligne.getByText(/Enregistré/)).toBeVisible({ timeout: 20_000 });
  await ligne.scrollIntoViewIfNeeded();
  await capturer(direction, "159-parametres-saisie-des-maitres-allumee");

  // Le maître entre, change son mot de passe, et ne voit que ce qui le concerne.
  await page.getByLabel(texte("connexion.identifiant")).fill(MAITRE);
  await page.getByLabel(texte("connexion.mot_de_passe")).fill(MOT_DE_PASSE_MAITRE);
  await page.getByRole("button", { name: texte("connexion.valider") }).click();
  const changement = page.locator("form").first();
  await changement.getByLabel(texte("compte.mot_de_passe_actuel")).fill(MOT_DE_PASSE_MAITRE);
  await changement.getByLabel(texte("compte.nouveau_mot_de_passe")).fill("maitre-mot-de-passe-change");
  await changement.getByRole("button", { name: texte("commun.enregistrer") }).click();
  await expect(page.getByText(texte("compte.pin_absent"))).toBeVisible({ timeout: 20_000 });
  const navigation = page.getByRole("navigation");
  await allerA(page, "nav.classes");
  for (const cle of ["nav.eleves", "nav.notes", "nav.absences", "nav.bulletins", "nav.classes", "nav.compte"] as const) {
    await expect(navigation.getByRole("button", { name: texte(cle), exact: true })).toBeVisible();
  }
  for (const cle of ["nav.guichet", "nav.inscriptions", "nav.caisse", "nav.frais", "nav.convocations", "nav.remises", "nav.utilisateurs", "nav.parametres", "nav.journal"] as const) {
    await expect(navigation.getByRole("button", { name: texte(cle), exact: true })).toHaveCount(0);
  }
  // Les élèves : ceux de sa classe, leur fiche sans la situation financière.
  await allerA(page, "nav.eleves");
  await expect(page.getByRole("cell", { name: "Fatoumata", exact: true })).toBeVisible({ timeout: 20_000 });
  await expect(page.getByRole("cell", { name: "Aïssata", exact: true })).toHaveCount(0);
  await page.locator("#eleves-recherche").fill("Keita");
  await expect(page.getByRole("cell", { name: "Fatoumata", exact: true })).toHaveCount(0);
  await expect(page.getByRole("cell", { name: "Aïssata", exact: true })).toHaveCount(0);
  await page.locator("#eleves-recherche").fill("");
  await page.getByRole("cell", { name: "Fatoumata", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Fatoumata Diallo" })).toBeVisible({ timeout: 20_000 });
  await expect(page.getByRole("heading", { name: texte("responsables.titre") })).toBeVisible();
  await expect(page.getByText("+223 76 45 12 90")).toBeVisible();
  await expect(page.getByRole("heading", { name: texte("facture.titre"), exact: true })).toHaveCount(0);
  await expect(page.getByRole("alert")).toHaveCount(0);
  await capturer(page, "160-maitre-eleves-de-sa-classe");
  // Ses notes : l'écran s'ouvre sur sa classe.
  await allerA(page, "nav.notes");
  await expect(page.getByRole("heading", { name: texte("notes.titre") })).toBeVisible({ timeout: 20_000 });
  await expect(page.getByRole("alert")).toHaveCount(0);
  await capturer(page, "161-maitre-notes");

  // La direction éteint le réglage : la session du maître se ferme à son geste suivant.
  await ligne.scrollIntoViewIfNeeded();
  await reglage.uncheck();
  await ligne.getByRole("button", { name: texte("commun.enregistrer") }).click();
  await expect(reglage).not.toBeChecked();
  await expect(async () => {
    await navigation.getByRole("button", { name: texte("nav.accueil"), exact: true }).click({ timeout: 2_000 });
    await expect(page.getByRole("button", { name: texte("connexion.valider") })).toBeVisible({ timeout: 2_000 });
  }).toPass({ timeout: 30_000 });

  // La directrice reprend le poste relié, pour la suite.
  await page.getByLabel(texte("connexion.identifiant")).fill(IDENTIFIANT);
  await page.getByLabel(texte("connexion.mot_de_passe")).fill(NOUVEAU_MOT_DE_PASSE);
  await page.getByRole("button", { name: texte("connexion.valider") }).click();
  await expect(navigation.getByRole("button", { name: texte("nav.accueil"), exact: true })).toBeVisible({ timeout: 20_000 });
  await expect(page.getByText(texte("bandeau.utilisateur", { nom: NOM }))).toBeVisible();
  await allerA(direction, "nav.postes");
});

test("poste principal éteint : la saisie reste, puis le poste reprend seul", async () => {
  const page = relie.page;
  await allerA(page, "nav.compte");
  const motDePasse = page.locator("form").first();
  await motDePasse.getByLabel(texte("compte.mot_de_passe_actuel")).fill(NOUVEAU_MOT_DE_PASSE);
  await motDePasse.getByLabel(texte("compte.nouveau_mot_de_passe")).fill("saisie-gardee-a-l-ecran");

  // La direction éteint le réseau : le poste principal ne répond plus.
  await allerA(principal.page, "nav.postes");
  await principal.page.getByRole("button", { name: texte("postes.eteindre"), exact: true }).click();
  await principal.page.getByRole("button", { name: texte("postes.eteindre_confirmer") }).click();
  await expect(principal.page.getByText(texte("postes.reseau.eteint"))).toBeVisible();
  await expect(async () => expect(await portOuvert()).toBe(false)).toPass({ timeout: 10_000 });

  // L'enregistrement échoue, le bandeau le dit, et rien de ce qui est saisi ne disparaît.
  await motDePasse.getByRole("button", { name: texte("commun.enregistrer") }).click();
  await expect(page.getByText(texte("bandeau_poste.coupee"))).toBeVisible({ timeout: 20_000 });
  await expect(motDePasse.getByRole("alert")).toContainText(texte("reseau.poste_principal_injoignable").slice(0, 40));
  await expect(motDePasse.getByLabel(texte("compte.nouveau_mot_de_passe"))).toHaveValue("saisie-gardee-a-l-ecran");
  await capturer(page, "151-poste-principal-injoignable");

  // Le réseau revient : sans rien toucher sur le poste relié, la liaison se rétablit.
  await principal.page.getByRole("button", { name: texte("postes.activer") }).click();
  await expect(principal.page.getByText(texte("postes.reseau.allume"))).toBeVisible();
  await expect(page.getByText(texte("bandeau_poste.etablie"))).toBeVisible({ timeout: 30_000 });
  await expect(motDePasse.getByLabel(texte("compte.nouveau_mot_de_passe"))).toHaveValue("saisie-gardee-a-l-ecran");
  // La session a survécu : le poste principal n'a pas redémarré.
  await allerA(page, "nav.journal");
  await expect(page.getByRole("heading", { name: texte("journal.titre") })).toBeVisible();
  await expect(page.getByRole("cell", { name: POSTE, exact: true }).first()).toBeVisible({ timeout: 20_000 });
});

test("le poste révoqué ne travaille plus, quitte le poste principal, et le journal garde tout", async () => {
  const page = principal.page;
  await page.getByRole("button", { name: texte("postes.revoquer_nom", { nom: POSTE }) }).click();
  await capturer(page, "144-postes-revoquer-confirmation");
  await page.getByRole("button", { name: texte("postes.revoquer_confirmer", { nom: POSTE }) }).click();
  const ligne = page.getByRole("row", { name: new RegExp(`^${POSTE}`) });
  await expect(ligne.getByText(/^Révoqué le /)).toBeVisible();
  // Sa place est rendue : on peut relier un autre poste.
  await expect(page.getByRole("button", { name: texte("postes.code_demander") })).toBeEnabled();
  await capturer(page, "145-postes-poste-revoque");

  // Sur le poste révoqué, la première commande est refusée : l'écran dit quoi faire.
  await allerA(relie.page, "nav.utilisateurs");
  await expect(relie.page.getByRole("heading", { name: texte("liaison.panne_titre") })).toBeVisible({ timeout: 20_000 });
  await expect(relie.page.getByRole("alert")).toHaveText(texte("reseau.poste_inconnu"));
  await expect(relie.page.getByText(texte("bandeau_poste.refusee"))).toBeVisible();
  await capturer(relie.page, "152-poste-revoque-sur-le-poste");
  // « Quitter le poste principal » ramène au premier lancement, sans liaison ni secret.
  await relie.page.getByRole("button", { name: texte("quitter_poste.lien") }).click();
  await relie.page.getByRole("button", { name: texte("quitter_poste.confirmer") }).click();
  await expect(relie.page.getByRole("heading", { name: texte("demarrage.titre") })).toBeVisible({ timeout: 30_000 });
  await expect(relie.page.getByText(texte("bandeau_poste.nom", { nom: POSTE }))).toHaveCount(0);
  expect(existsSync(join(relie.donnees, "poste.json"))).toBe(false);
  expect(existsSync(join(relie.donnees, "trousseau-essai", "liaison.json"))).toBe(false);

  await allerA(page, "nav.journal");
  await expect(page.getByRole("heading", { name: texte("journal.titre") })).toBeVisible();
  // Les lignes viennent du noyau, pas du réseau de la page : on attend de les voir.
  await expect(page.getByRole("cell", { name: texte("entite.poste_relie"), exact: true }).first()).toBeVisible({ timeout: 20_000 });
  await expect(page.getByRole("cell", { name: texte("entite.reseau_local"), exact: true }).first()).toBeVisible();
  await capturer(page, "146-postes-journal");

  // Réseau éteint : le port ne répond plus.
  await allerA(page, "nav.postes");
  await page.getByRole("button", { name: texte("postes.eteindre"), exact: true }).click();
  await page.getByRole("button", { name: texte("postes.eteindre_confirmer") }).click();
  await expect(page.getByText(texte("postes.reseau.eteint"))).toBeVisible();
  await expect(async () => expect(await portOuvert()).toBe(false)).toPass({ timeout: 10_000 });
});
