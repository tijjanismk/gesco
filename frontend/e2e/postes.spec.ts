// Postes de l'école, sur la vraie application : la direction allume le réseau, affiche un code,
// un poste se relie par le canal chiffré et ouvre une session, puis il est révoqué. Le poste
// à relier est joué par cet essai lui-même : l'écran de ce poste viendra avec l'étape suivante.
// En mode d'essai, le serveur n'écoute que la machine elle-même.
import { type ChildProcess, spawn, spawnSync } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { request } from "node:https";
import { tmpdir } from "node:os";
import { join } from "node:path";
import type { TLSSocket } from "node:tls";

import { type Browser, type Page, chromium, expect, test } from "@playwright/test";

import { catalogue, type Cle } from "../src/i18n/fr-ML";

const EXECUTABLE = join(import.meta.dirname, "..", "..", "target", "debug", "gesco-app.exe");
const CAPTURES = join(import.meta.dirname, "captures");
const PORT_DEBOGAGE = 9223;
const PORT_RESEAU = 47800;
const IDENTIFIANT = "direction";
const NOM = "Awa Traoré";
const MOT_DE_PASSE = "essai-premier-demarrage";
const NOUVEAU_MOT_DE_PASSE = "essai-mot-de-passe-change";
const POSTE = "Caisse";

const texte = (cle: Cle) => catalogue[cle];

let application: ChildProcess | undefined;
let navigateur: Browser | undefined;
let page: Page;
let dossier: string;
/** Ce que le poste relié garde après son appairage. */
let relie: { poste_id: string; secret: string };

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

const DEPLIER = "html, body, #racine, .h-screen { height: auto !important; } main { overflow: visible !important; }";

async function capturer(nom: string) {
  const depli = await page.addStyleTag({ content: DEPLIER });
  await page.screenshot({ path: join(CAPTURES, `${nom}.png`), fullPage: true });
  await depli.evaluate((style) => style.parentNode?.removeChild(style));
}

async function allerA(cle: Cle) {
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

interface Reponse {
  statut: number;
  corps: Record<string, unknown>;
  /** Empreinte courte du certificat présenté par le poste principal. */
  empreinte: string;
}

/** Appel d'un poste au poste principal, par le canal chiffré. Le certificat n'est signé par
 *  personne : on relève son empreinte, que l'essai compare à celle de l'écran. */
function appelerPrincipal(chemin: string, corps: unknown, entetes: Record<string, string> = {}): Promise<Reponse> {
  const donnees = JSON.stringify(corps);
  return new Promise((resoudre, rejeter) => {
    const requete = request(
      {
        host: "127.0.0.1",
        port: PORT_RESEAU,
        path: chemin,
        method: "POST",
        rejectUnauthorized: false,
        agent: false,
        headers: { "content-type": "application/json", "content-length": Buffer.byteLength(donnees), ...entetes },
      },
      (reponse) => {
        const complete = (reponse.socket as TLSSocket).getPeerCertificate().fingerprint256.replaceAll(":", "");
        const empreinte = (complete.slice(0, 16).match(/.{4}/g) ?? []).join("-");
        let recu = "";
        reponse.setEncoding("utf8");
        reponse.on("data", (morceau: string) => (recu += morceau));
        reponse.on("end", () => resoudre({ statut: reponse.statusCode ?? 0, corps: JSON.parse(recu) as Record<string, unknown>, empreinte }));
      },
    );
    requete.on("error", rejeter);
    requete.end(donnees);
  });
}

const identite = () => ({ "x-gesco-poste": relie.poste_id, "x-gesco-secret": relie.secret });

test.describe.configure({ mode: "serial" });

test.beforeAll(async () => {
  dossier = mkdtempSync(join(tmpdir(), "gesco-postes-"));
  application = spawn(EXECUTABLE, [], {
    env: {
      ...process.env,
      GESCO_DOSSIER_ESSAI: join(dossier, "donnees"),
      GESCO_DATE_ESSAI: "2026-10-14",
      WEBVIEW2_USER_DATA_FOLDER: join(dossier, "webview"),
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${PORT_DEBOGAGE}`,
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
  if (application?.pid !== undefined) spawnSync("taskkill", ["/pid", String(application.pid), "/T", "/F"]);
  try {
    rmSync(dossier, { recursive: true, force: true, maxRetries: 10, retryDelay: 500 });
  } catch {
    return;
  }
});

test("une école d'essai et sa directrice", async () => {
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

  await page.getByLabel(texte("connexion.identifiant")).fill(IDENTIFIANT);
  await page.getByLabel(texte("connexion.mot_de_passe")).fill(MOT_DE_PASSE);
  await page.getByRole("button", { name: texte("connexion.valider") }).click();
  await expect(page.getByText(texte("compte.changement_exige"))).toBeVisible();
  const motDePasse = page.locator("form").first();
  await motDePasse.getByLabel(texte("compte.mot_de_passe_actuel")).fill(MOT_DE_PASSE);
  await motDePasse.getByLabel(texte("compte.nouveau_mot_de_passe")).fill(NOUVEAU_MOT_DE_PASSE);
  await motDePasse.getByRole("button", { name: texte("commun.enregistrer") }).click();
  await expect(page.getByText(texte("compte.changement_exige"))).toBeHidden();
});

test("la direction allume le réseau de l'école", async () => {
  await allerA("nav.postes");
  await expect(page.getByRole("heading", { name: texte("postes.titre"), level: 1 })).toBeVisible();
  await expect(page.getByText(texte("postes.reseau.eteint"))).toBeVisible();
  await expect(page.getByText(texte("postes.liste_vide"))).toBeVisible();
  await capturer("140-postes-reseau-eteint");

  // Tant que le réseau est éteint, personne ne répond sur le port.
  await expect(appelerPrincipal("/api/appairage", { code: "AAAAAAAA", nom: POSTE })).rejects.toThrow();

  await page.getByRole("button", { name: texte("postes.activer") }).click();
  await expect(page.getByText(texte("postes.reseau.allume"))).toBeVisible();
  // Le serveur fabrique le certificat à la première activation : son empreinte paraît à la
  // relecture suivante de l'écran.
  await expect(page.getByText(texte("postes.empreinte_attente"))).toBeHidden({ timeout: 20_000 });
  await expect(page.getByText(/^[0-9A-F]{4}(-[0-9A-F]{4}){3}$/)).toBeVisible();
  await expect(page.getByText("127.0.0.1", { exact: true })).toBeVisible();
  await capturer("141-postes-reseau-allume");
});

test("un poste se relie par le code affiché et ouvre une session", async () => {
  await page.getByRole("button", { name: texte("postes.code_demander") }).click();
  const zoneCode = page.getByRole("status");
  await expect(zoneCode).toHaveText(/^[A-Z2-9]{4} [A-Z2-9]{4}$/);
  const code = (await zoneCode.innerText()).trim();
  const empreinteAffichee = (await page.getByText(/^[0-9A-F]{4}(-[0-9A-F]{4}){3}$/).innerText()).trim();
  await capturer("142-postes-code-a-saisir");

  // Un code faux ne relie rien ; le bon, saisi en minuscules et sans espace, relie le poste.
  const faux = await appelerPrincipal("/api/appairage", { code: "ZZZZZZZZ", nom: POSTE });
  expect(faux.statut).toBe(422);
  expect(faux.corps["cle"]).toBe("reseau.code_faux");
  const conclu = await appelerPrincipal("/api/appairage", { code: code.replace(" ", "").toLowerCase(), nom: POSTE });
  expect(conclu.statut).toBe(200);
  // Le certificat que le poste a vu est bien celui dont l'écran montre l'empreinte.
  expect(conclu.empreinte).toBe(empreinteAffichee);
  relie = { poste_id: String(conclu.corps["poste_id"]), secret: String(conclu.corps["secret"]) };

  // Sans son secret, un appel est refusé ; avec lui, le poste ouvre une session sous son nom.
  const anonyme = await appelerPrincipal("/api/commandes/etat_demarrage", null);
  expect(anonyme.statut).toBe(401);
  const session = await appelerPrincipal("/api/commandes/connexion", { identifiant: IDENTIFIANT, mot_de_passe: NOUVEAU_MOT_DE_PASSE }, identite());
  expect(session.statut).toBe(200);
  expect(session.corps["poste"]).toBe(POSTE);

  // L'écran se relit de lui-même : le poste paraît, sa session ouverte ; le code a servi.
  const ligne = page.getByRole("row", { name: new RegExp(`^${POSTE}`) });
  await expect(ligne).toBeVisible({ timeout: 20_000 });
  await expect(ligne.getByText(texte("postes.etat.connecte"))).toBeVisible({ timeout: 20_000 });
  // Sans licence, un poste relié à l'essai : il est pris, on ne peut pas en relier un autre.
  await expect(page.getByText(texte("postes.complet"))).toBeVisible();
  await expect(page.getByRole("button", { name: texte("postes.code_demander") })).toBeDisabled();
  await capturer("143-postes-poste-relie");
});

test("le poste révoqué ne travaille plus, et le journal garde tout", async () => {
  await page.getByRole("button", { name: texte("postes.revoquer_nom").replace("{nom}", POSTE) }).click();
  await capturer("144-postes-revoquer-confirmation");
  await page.getByRole("button", { name: texte("postes.revoquer_confirmer").replace("{nom}", POSTE) }).click();
  const ligne = page.getByRole("row", { name: new RegExp(`^${POSTE}`) });
  await expect(ligne.getByText(/^Révoqué le /)).toBeVisible();
  const refuse = await appelerPrincipal("/api/commandes/etat_demarrage", null, identite());
  expect(refuse.statut).toBe(401);
  expect(refuse.corps["cle"]).toBe("reseau.poste_inconnu");
  // Sa place est rendue : on peut relier un autre poste.
  await expect(page.getByRole("button", { name: texte("postes.code_demander") })).toBeEnabled();
  await capturer("145-postes-poste-revoque");

  await allerA("nav.journal");
  await expect(page.getByRole("heading", { name: texte("journal.titre") })).toBeVisible();
  // Les lignes viennent du noyau, pas du réseau de la page : on attend de les voir.
  await expect(page.getByRole("cell", { name: texte("entite.poste_relie"), exact: true }).first()).toBeVisible({ timeout: 20_000 });
  await expect(page.getByRole("cell", { name: texte("entite.reseau_local"), exact: true }).first()).toBeVisible();
  await capturer("146-postes-journal");

  // Réseau éteint : le port ne répond plus.
  await allerA("nav.postes");
  await page.getByRole("button", { name: texte("postes.eteindre"), exact: true }).click();
  await page.getByRole("button", { name: texte("postes.eteindre_confirmer") }).click();
  await expect(page.getByText(texte("postes.reseau.eteint"))).toBeVisible();
  await expect(async () => {
    await expect(appelerPrincipal("/api/appairage", { code: "AAAAAAAA", nom: POSTE })).rejects.toThrow();
  }).toPass({ timeout: 10_000 });
});
