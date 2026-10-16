import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { choisirPiece, enregistrerPieceSous } from "../../api/fichiers";
import { commandeAutorisee } from "../../app/navigation";
import { Bouton } from "../../composants/Bouton";
import { CaseACocher, Champ, Selection } from "../../composants/Champ";
import { MessageErreur, Section } from "../../composants/Messages";
import { Tableau } from "../../composants/Tableau";
import { type Cle, dateCivile, t } from "../../i18n";
import type {
  CategoriePersonnel,
  Diplome,
  FicheMembre,
  FonctionPersonnel,
  MembreAffiche,
  MoyenPaiement,
  NaturePiece,
  Sexe,
  SituationFamille,
  StatutEnseignant,
} from "../../types/gesco";
import { telephoneLisible } from "../scolarite/Identite";
import { EngagementsMembre } from "./Engagements";

const NATURES: Record<NaturePiece, Cle> = {
  identite: "personnel.piece.identite",
  diplome: "personnel.piece.diplome",
  engagement: "personnel.piece.engagement",
  autre: "personnel.piece.autre",
};

const MODES: Record<MoyenPaiement["mode"], Cle> = {
  especes: "personnel.paiement.especes",
  mobile_money: "personnel.paiement.mobile_money",
  banque: "personnel.paiement.banque",
};

const SITUATIONS: Record<SituationFamille, Cle> = {
  celibataire: "personnel.situation.celibataire",
  marie: "personnel.situation.marie",
  veuf: "personnel.situation.veuf",
  divorce: "personnel.situation.divorce",
};

const AUCUN = "";

function useRafraichir() {
  const client = useQueryClient();
  return () => {
    for (const cle of ["personnel", "membre", "personnel-referentiel", "enseignants"]) void client.invalidateQueries({ queryKey: [cle] });
  };
}

/** Registre du personnel (SPEC_PHASE_6 §2) : tout le personnel de l'école, sa fiche et son dossier. */
export function Personnel() {
  const session = useQuery({ queryKey: ["session"], queryFn: () => appeler("session_courante", null) });
  const membres = useQuery({ queryKey: ["personnel"], queryFn: () => appeler("personnel_lister", null) });
  const referentiel = useQuery({ queryKey: ["personnel-referentiel"], queryFn: () => appeler("personnel_referentiel_lire", null) });
  const [texte, setTexte] = useState("");
  const [partis, setPartis] = useState(false);
  const [choisi, setChoisi] = useState<string | null>(null);
  const [creation, setCreation] = useState(false);
  const permissions = session.data?.permissions ?? [];
  const peutModifier = commandeAutorisee("membre_enregistrer", permissions);
  const cherche = texte.trim().toLowerCase();
  const lignes = (membres.data ?? []).filter(
    (m) =>
      (partis || m.membre.parti_le === null) &&
      (cherche === "" || `${m.prenoms} ${m.nom} ${m.membre.matricule} ${m.membre.fonction}`.toLowerCase().includes(cherche)),
  );
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("personnel.titre")}</h1>
      <div className="flex flex-wrap items-end gap-3">
        <div className="w-80">
          <Champ id="personnel-recherche" type="search" libelle={t("personnel.rechercher")} value={texte} onChange={(e) => setTexte(e.target.value)} />
        </div>
        {peutModifier ? (
          <Bouton
            onClick={() => {
              setCreation(true);
              setChoisi(null);
            }}
          >
            {t("personnel.nouveau")}
          </Bouton>
        ) : null}
        <CaseACocher id="personnel-partis" libelle={t("personnel.afficher_partis")} coche={partis} surChangement={setPartis} />
      </div>
      <MessageErreur erreur={membres.error} />
      <div className="flex flex-wrap items-start gap-4">
        <div className="min-w-0 flex-1">
          <Tableau
            nomExport="personnel"
            titre={t("personnel.titre")}
            lignes={lignes}
            cle={(m) => m.membre.id}
            surSelection={(m) => {
              setChoisi(m.membre.id);
              setCreation(false);
            }}
            colonnes={[
              { titre: t("personnel.matricule"), valeur: (m) => m.membre.matricule },
              { titre: t("personnel.prenoms_nom"), valeur: (m) => `${m.prenoms} ${m.nom}` },
              { titre: t("personnel.fonction"), valeur: (m) => m.membre.fonction },
              { titre: t("personne.telephone"), valeur: (m) => m.telephones.map(telephoneLisible).join(", ") },
              { titre: t("personnel.etat"), valeur: etat },
            ]}
          />
        </div>
        <aside className="w-full max-w-2xl">
          {creation && referentiel.data !== undefined ? (
            <FormulaireMembre
              fiche={null}
              categories={referentiel.data.categories}
              fonctions={referentiel.data.fonctions}
              permissions={permissions}
              surEnregistre={(id) => {
                setCreation(false);
                setChoisi(id);
              }}
              surAnnuler={() => setCreation(false)}
            />
          ) : null}
          {choisi === null || referentiel.data === undefined ? null : (
            <FicheMembreEcran key={choisi} id={choisi} categories={referentiel.data.categories} fonctions={referentiel.data.fonctions} permissions={permissions} />
          )}
        </aside>
      </div>
      {peutModifier && referentiel.data !== undefined ? <CategoriesEtFonctions categories={referentiel.data.categories} fonctions={referentiel.data.fonctions} /> : null}
    </div>
  );
}

function etat(m: MembreAffiche): string {
  if (m.membre.parti_le !== null) return t("personnel.parti_le", { date: dateCivile(m.membre.parti_le) });
  return t(m.enseignant ? "personnel.present_enseignant" : "personnel.present");
}

function FicheMembreEcran({
  id,
  categories,
  fonctions,
  permissions,
}: {
  id: string;
  categories: readonly CategoriePersonnel[];
  fonctions: readonly FonctionPersonnel[];
  permissions: readonly string[];
}) {
  const fiche = useQuery({ queryKey: ["membre", id], queryFn: () => appeler("membre_lire", { id }) });
  // Le formulaire renaît avec la fiche relue : la confirmation vit ici, au-dessus de lui.
  const [enregistre, setEnregistre] = useState(false);
  if (fiche.data === undefined) return <MessageErreur erreur={fiche.error} />;
  const f = fiche.data;
  return (
    <div className="flex flex-col gap-4">
      <h2 className="text-lg font-semibold text-encre">{`${f.personne.prenoms} ${f.personne.nom} · ${f.membre.matricule}`}</h2>
      {f.membre.parti_le === null ? null : (
        <p className="text-sm text-ocre">{t("personnel.parti_motif", { date: dateCivile(f.membre.parti_le), motif: f.membre.motif_depart })}</p>
      )}
      <FormulaireMembre key={JSON.stringify(f.membre)} fiche={f} categories={categories} fonctions={fonctions} permissions={permissions} surEnregistre={() => setEnregistre(true)} surAnnuler={null} />
      {enregistre ? <p role="status" className="text-sm text-vert">{t("personnel.enregistre")}</p> : null}
      {commandeAutorisee("engagements_lister", permissions) ? <EngagementsMembre fiche={f} permissions={permissions} /> : null}
      <PiecesDossier fiche={f} permissions={permissions} />
      {commandeAutorisee("membre_partir", permissions) ? <AutresActions fiche={f} permissions={permissions} /> : null}
    </div>
  );
}

interface Saisie {
  prenoms: string;
  nom: string;
  sexe: Sexe | null;
  date_naissance: string;
  telephone: string;
  adresse: string;
  matricule: string;
  categorie_id: string;
  fonction: string;
  arrive_le: string;
  fonctionnaire: boolean;
  numero_fonctionnaire: string;
  numero_inps: string;
  prevenir_nom: string;
  prevenir_telephone: string;
  mode: MoyenPaiement["mode"];
  numero: string;
  banque: string;
  compte_bancaire: string;
  diplomes: Diplome[];
  compte: string;
  situation: SituationFamille | null;
  enfants: string;
}

function saisieDe(fiche: FicheMembre | null, categories: readonly CategoriePersonnel[]): Saisie {
  if (fiche === null) {
    return {
      prenoms: "",
      nom: "",
      sexe: null,
      date_naissance: "",
      telephone: "",
      adresse: "",
      matricule: "",
      categorie_id: categories.find((c) => c.active)?.id ?? "",
      fonction: "",
      arrive_le: "",
      fonctionnaire: false,
      numero_fonctionnaire: "",
      numero_inps: "",
      prevenir_nom: "",
      prevenir_telephone: "",
      mode: "especes",
      numero: "",
      banque: "",
      compte_bancaire: "",
      diplomes: [],
      compte: AUCUN,
      situation: null,
      enfants: "",
    };
  }
  const m = fiche.membre;
  const p = m.paiement;
  return {
    prenoms: fiche.personne.prenoms,
    nom: fiche.personne.nom,
    sexe: fiche.personne.sexe,
    date_naissance: fiche.personne.date_naissance ?? "",
    telephone: fiche.personne.telephones[0] === undefined ? "" : telephoneLisible(fiche.personne.telephones[0]),
    adresse: fiche.personne.adresse,
    matricule: m.matricule,
    categorie_id: m.categorie_id,
    fonction: m.fonction,
    arrive_le: m.arrive_le ?? "",
    fonctionnaire: m.fonctionnaire,
    numero_fonctionnaire: m.numero_fonctionnaire,
    numero_inps: m.numero_inps,
    prevenir_nom: m.prevenir_nom,
    prevenir_telephone: m.prevenir_telephone === "" ? "" : telephoneLisible(m.prevenir_telephone),
    mode: p.mode,
    numero: p.mode === "mobile_money" ? telephoneLisible(p.numero) : "",
    banque: p.mode === "banque" ? p.banque : "",
    compte_bancaire: p.mode === "banque" ? p.compte : "",
    diplomes: m.diplomes,
    compte: m.utilisateur_id ?? AUCUN,
    situation: m.situation_famille,
    enfants: m.enfants_a_charge === null ? "" : String(m.enfants_a_charge),
  };
}

function paiementDe(s: Saisie): MoyenPaiement {
  if (s.mode === "mobile_money") return { mode: "mobile_money", numero: s.numero };
  if (s.mode === "banque") return { mode: "banque", banque: s.banque, compte: s.compte_bancaire };
  return { mode: "especes" };
}

function FormulaireMembre({
  fiche,
  categories,
  fonctions,
  permissions,
  surEnregistre,
  surAnnuler,
}: {
  fiche: FicheMembre | null;
  categories: readonly CategoriePersonnel[];
  fonctions: readonly FonctionPersonnel[];
  permissions: readonly string[];
  surEnregistre: (id: string) => void;
  surAnnuler: (() => void) | null;
}) {
  const rafraichir = useRafraichir();
  const voitComptes = commandeAutorisee("utilisateurs_lister", permissions);
  const modifiable = commandeAutorisee("membre_enregistrer", permissions);
  const comptes = useQuery({ queryKey: ["utilisateurs"], queryFn: () => appeler("utilisateurs_lister", null), enabled: voitComptes });
  const [s, setS] = useState<Saisie>(() => saisieDe(fiche, categories));
  const maj = (champ: Partial<Saisie>) => setS((x) => ({ ...x, ...champ }));
  const enregistrer = useMutation({
    mutationFn: () =>
      appeler("membre_enregistrer", {
        id: fiche?.membre.id ?? null,
        nom: s.nom,
        prenoms: s.prenoms,
        sexe: s.sexe,
        date_naissance: s.date_naissance === "" ? null : s.date_naissance,
        telephones: s.telephone.trim() === "" ? [] : [s.telephone],
        adresse: s.adresse,
        matricule: s.matricule,
        categorie_id: s.categorie_id,
        fonction: s.fonction,
        arrive_le: s.arrive_le === "" ? null : s.arrive_le,
        fonctionnaire: s.fonctionnaire,
        numero_fonctionnaire: s.numero_fonctionnaire,
        numero_inps: s.numero_inps,
        prevenir_nom: s.prevenir_nom,
        prevenir_telephone: s.prevenir_telephone,
        paiement: paiementDe(s),
        diplomes: s.diplomes,
        utilisateur_id: s.compte === AUCUN ? null : s.compte,
        situation_famille: s.situation,
        enfants_a_charge: s.enfants.trim() === "" ? null : Number.parseInt(s.enfants.replace(/\D/g, "") || "0", 10),
      }),
    onSuccess: (f) => {
      rafraichir();
      surEnregistre(f.membre.id);
    },
  });
  // Une catégorie retirée reste proposée à la fiche qui la porte.
  const categoriesProposees = categories.filter((c) => c.active || c.id === fiche?.membre.categorie_id);
  const fonctionsProposees = fonctions.filter((f) => f.active && f.categorie_id === s.categorie_id);
  const diplome = (i: number, champ: Partial<Diplome>) => maj({ diplomes: s.diplomes.map((d, j) => (j === i ? { ...d, ...champ } : d)) });
  return (
    <Section titre={t(fiche === null ? "personnel.nouveau" : "personnel.fiche")}>
      {/* Sans `employees:update`, la fiche se lit sans pouvoir être changée. */}
      <fieldset disabled={!modifiable} className="flex min-w-0 flex-col gap-3">
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <Champ id="membre-prenoms" libelle={t("personne.prenoms")} value={s.prenoms} onChange={(e) => maj({ prenoms: e.target.value })} />
        <Champ id="membre-nom" libelle={t("personne.nom")} value={s.nom} onChange={(e) => maj({ nom: e.target.value })} />
        <Selection
          id="membre-sexe"
          libelle={t("personne.sexe")}
          valeur={s.sexe ?? AUCUN}
          surChangement={(v) => maj({ sexe: v === AUCUN ? null : (v as Sexe) })}
          options={[
            { valeur: AUCUN, libelle: t("personne.sexe.choisir") },
            { valeur: "masculin", libelle: t("personne.sexe.masculin") },
            { valeur: "feminin", libelle: t("personne.sexe.feminin") },
          ]}
        />
        <Champ id="membre-naissance" type="date" libelle={t("personne.date_naissance")} value={s.date_naissance} onChange={(e) => maj({ date_naissance: e.target.value })} />
        <Champ id="membre-telephone" type="tel" libelle={t("personne.telephone")} value={s.telephone} onChange={(e) => maj({ telephone: e.target.value })} />
        <Champ id="membre-adresse" libelle={t("personne.adresse")} value={s.adresse} onChange={(e) => maj({ adresse: e.target.value })} />
        <Selection
          id="membre-categorie"
          libelle={t("personnel.categorie")}
          valeur={s.categorie_id}
          surChangement={(v) => maj({ categorie_id: v })}
          options={categoriesProposees.map((c) => ({ valeur: c.id, libelle: c.libelle }))}
        />
        <div>
          <Champ id="membre-fonction" list="membre-fonctions" libelle={t("personnel.fonction")} aide={t("personnel.fonction_aide")} value={s.fonction} onChange={(e) => maj({ fonction: e.target.value })} />
          <datalist id="membre-fonctions">
            {fonctionsProposees.map((f) => (
              <option key={f.id} value={f.libelle} />
            ))}
          </datalist>
        </div>
        <Champ id="membre-matricule" libelle={t("personnel.matricule")} aide={t("personnel.matricule_aide")} value={s.matricule} onChange={(e) => maj({ matricule: e.target.value })} />
        <Champ id="membre-arrivee" type="date" libelle={t("personnel.arrive_le")} value={s.arrive_le} onChange={(e) => maj({ arrive_le: e.target.value })} />
        <Champ id="membre-inps" libelle={t("personnel.numero_inps")} value={s.numero_inps} onChange={(e) => maj({ numero_inps: e.target.value })} />
        <Selection
          id="membre-situation"
          libelle={t("personnel.situation_famille")}
          valeur={s.situation ?? AUCUN}
          surChangement={(v) => maj({ situation: v === AUCUN ? null : (v as SituationFamille) })}
          options={[{ valeur: AUCUN, libelle: t("personnel.situation.choisir") }, ...Object.entries(SITUATIONS).map(([v, c]) => ({ valeur: v, libelle: t(c) }))]}
        />
        <Champ id="membre-enfants" inputMode="numeric" libelle={t("personnel.enfants_a_charge")} value={s.enfants} onChange={(e) => maj({ enfants: e.target.value })} />
        <div className="flex flex-col gap-2">
          <CaseACocher id="membre-fonctionnaire" libelle={t("personnel.fonctionnaire")} coche={s.fonctionnaire} surChangement={(c) => maj({ fonctionnaire: c })} />
          {s.fonctionnaire ? (
            <Champ id="membre-numero-fonctionnaire" libelle={t("personnel.numero_fonctionnaire")} value={s.numero_fonctionnaire} onChange={(e) => maj({ numero_fonctionnaire: e.target.value })} />
          ) : null}
        </div>
        <Champ id="membre-prevenir-nom" libelle={t("personnel.prevenir_nom")} value={s.prevenir_nom} onChange={(e) => maj({ prevenir_nom: e.target.value })} />
        <Champ id="membre-prevenir-telephone" type="tel" libelle={t("personnel.prevenir_telephone")} value={s.prevenir_telephone} onChange={(e) => maj({ prevenir_telephone: e.target.value })} />
        <Selection
          id="membre-paiement"
          libelle={t("personnel.paiement")}
          valeur={s.mode}
          surChangement={(v) => maj({ mode: v as MoyenPaiement["mode"] })}
          options={Object.entries(MODES).map(([v, c]) => ({ valeur: v, libelle: t(c) }))}
        />
        {s.mode === "mobile_money" ? <Champ id="membre-paiement-numero" type="tel" libelle={t("personnel.paiement.numero")} value={s.numero} onChange={(e) => maj({ numero: e.target.value })} /> : null}
        {s.mode === "banque" ? (
          <>
            <Champ id="membre-paiement-banque" libelle={t("personnel.paiement.nom_banque")} value={s.banque} onChange={(e) => maj({ banque: e.target.value })} />
            <Champ id="membre-paiement-compte" libelle={t("personnel.paiement.compte")} value={s.compte_bancaire} onChange={(e) => maj({ compte_bancaire: e.target.value })} />
          </>
        ) : null}
        {voitComptes ? (
          <Selection
            id="membre-compte"
            libelle={t("personnel.compte")}
            valeur={s.compte}
            surChangement={(c) => maj({ compte: c })}
            options={[{ valeur: AUCUN, libelle: t("personnel.sans_compte") }, ...(comptes.data ?? []).map((u) => ({ valeur: u.utilisateur.id, libelle: u.utilisateur.nom_affiche }))]}
          />
        ) : null}
      </div>
      <h3 className="text-base font-semibold text-encre">{t("personnel.diplomes")}</h3>
      {s.diplomes.map((d, i) => (
        <div key={i} className="flex flex-wrap items-end gap-2">
          <Champ id={`diplome-${i}-intitule`} libelle={t("personnel.diplome.intitule")} value={d.intitule} onChange={(e) => diplome(i, { intitule: e.target.value })} />
          <Champ id={`diplome-${i}-delivre`} libelle={t("personnel.diplome.delivre_par")} value={d.delivre_par} onChange={(e) => diplome(i, { delivre_par: e.target.value })} />
          <div className="w-24">
            <Champ
              id={`diplome-${i}-annee`}
              inputMode="numeric"
              libelle={t("personnel.diplome.annee")}
              value={d.annee === null ? "" : String(d.annee)}
              onChange={(e) => diplome(i, { annee: e.target.value.trim() === "" ? null : Number(e.target.value) })}
            />
          </div>
          <Bouton variante="secondaire" onClick={() => maj({ diplomes: s.diplomes.filter((_, j) => j !== i) })}>{t("commun.retirer")}</Bouton>
        </div>
      ))}
      </fieldset>
      {modifiable ? (
        <div>
          <Bouton variante="secondaire" onClick={() => maj({ diplomes: [...s.diplomes, { intitule: "", delivre_par: "", annee: null }] })}>{t("personnel.diplome.ajouter")}</Bouton>
        </div>
      ) : null}
      <MessageErreur erreur={enregistrer.error} />
      {modifiable ? (
        <div className="flex flex-wrap gap-2">
          <Bouton onClick={() => enregistrer.mutate()} disabled={enregistrer.isPending}>{t("commun.enregistrer")}</Bouton>
          {surAnnuler === null ? null : <Bouton variante="secondaire" onClick={surAnnuler}>{t("commun.annuler")}</Bouton>}
        </div>
      ) : null}
    </Section>
  );
}

function PiecesDossier({ fiche, permissions }: { fiche: FicheMembre; permissions: readonly string[] }) {
  const rafraichir = useRafraichir();
  const [intitule, setIntitule] = useState("");
  const [nature, setNature] = useState<NaturePiece>("identite");
  const [retour, setRetour] = useState<string | null>(null);
  const ajouter = useMutation({
    mutationFn: async () => {
      const chemin = await choisirPiece(t("personnel.piece.filtre"));
      if (chemin === null) return null;
      return appeler("piece_dossier_ajouter", { membre_id: fiche.membre.id, intitule, nature, chemin });
    },
    onSuccess: (piece) => {
      if (piece === null) return;
      setIntitule("");
      setRetour(t("personnel.piece.ajoutee", { intitule: piece.intitule }));
      rafraichir();
    },
  });
  const ouvrir = useMutation({
    mutationFn: async ({ id, nom }: { id: string; nom: string }) => {
      const chemin = await enregistrerPieceSous(nom);
      if (chemin === null) return null;
      await appeler("piece_dossier_enregistrer_sous", { id, chemin });
      return chemin;
    },
    onSuccess: (chemin) => {
      if (chemin !== null) setRetour(t("personnel.piece.enregistree_sous", { chemin }));
    },
  });
  const retirer = useMutation({
    mutationFn: (id: string) => appeler("piece_dossier_retirer", { id }),
    onSuccess: () => rafraichir(),
  });
  const modifiable = commandeAutorisee("piece_dossier_ajouter", permissions);
  const actives = fiche.pieces.filter((p) => p.active);
  return (
    <Section titre={t("personnel.pieces")}>
      {actives.length === 0 ? <p className="text-sm text-muet">{t("personnel.pieces_aucune")}</p> : null}
      <ul className="flex flex-col gap-2">
        {actives.map((p) => (
          <li key={p.id} className="flex flex-wrap items-center gap-2 text-sm text-encre">
            <span className="font-medium">{p.intitule}</span>
            <span className="text-muet">{`${t(NATURES[p.nature])} · ${p.nom_fichier} · ${t("personnel.piece.ko", { taille: Math.max(1, Math.round(p.taille / 1024)) })}`}</span>
            <Bouton variante="secondaire" onClick={() => ouvrir.mutate({ id: p.id, nom: p.nom_fichier })}>{t("personnel.piece.enregistrer_sous")}</Bouton>
            {modifiable ? <Bouton variante="secondaire" onClick={() => retirer.mutate(p.id)}>{t("commun.retirer")}</Bouton> : null}
          </li>
        ))}
      </ul>
      {modifiable ? (
        <div className="flex flex-wrap items-end gap-2">
          <Champ id="piece-intitule" libelle={t("personnel.piece.intitule")} value={intitule} onChange={(e) => setIntitule(e.target.value)} />
          <Selection id="piece-nature" libelle={t("personnel.piece.nature")} valeur={nature} surChangement={(v) => setNature(v as NaturePiece)} options={Object.entries(NATURES).map(([v, c]) => ({ valeur: v, libelle: t(c) }))} />
          <Bouton variante="secondaire" onClick={() => ajouter.mutate()} disabled={ajouter.isPending}>{t("personnel.piece.ajouter")}</Bouton>
        </div>
      ) : null}
      <p className="text-sm text-muet">{t("personnel.piece.aide")}</p>
      {retour === null ? null : <p role="status" className="text-sm text-vert">{retour}</p>}
      <MessageErreur erreur={ajouter.error ?? ouvrir.error ?? retirer.error} />
    </Section>
  );
}

function AutresActions({ fiche, permissions }: { fiche: FicheMembre; permissions: readonly string[] }) {
  const rafraichir = useRafraichir();
  const [le, setLe] = useState("");
  const [motif, setMotif] = useState("");
  const [statut, setStatut] = useState<StatutEnseignant>("titulaire");
  const partir = useMutation({ mutationFn: () => appeler("membre_partir", { id: fiche.membre.id, le, motif }), onSuccess: rafraichir });
  const revenir = useMutation({ mutationFn: () => appeler("membre_revenir", { id: fiche.membre.id }), onSuccess: rafraichir });
  const enseigner = useMutation({ mutationFn: () => appeler("membre_enseignant", { id: fiche.membre.id, statut }), onSuccess: rafraichir });
  const enseigne = fiche.enseignant?.actif === true;
  return (
    <details className="rounded border border-muet/30 p-2">
      <summary className="cursor-pointer text-sm text-primaire">{t("personnel.autres_actions")}</summary>
      <div className="mt-2 flex flex-col gap-3">
        {fiche.membre.parti_le === null && !enseigne && commandeAutorisee("membre_enseignant", permissions) ? (
          <div className="flex flex-wrap items-end gap-2">
            <Selection
              id="membre-statut-enseignant"
              libelle={t("enseignants.statut")}
              valeur={statut}
              surChangement={(v) => setStatut(v as StatutEnseignant)}
              options={[
                { valeur: "titulaire", libelle: t("enseignant.statut.titulaire") },
                { valeur: "vacataire", libelle: t("enseignant.statut.vacataire") },
              ]}
            />
            <Bouton variante="secondaire" onClick={() => enseigner.mutate()} disabled={enseigner.isPending}>{t("personnel.devenir_enseignant")}</Bouton>
          </div>
        ) : null}
        {fiche.membre.parti_le === null ? (
          <div className="flex flex-wrap items-end gap-2">
            <Champ id="membre-depart-le" type="date" libelle={t("personnel.depart_le")} value={le} onChange={(e) => setLe(e.target.value)} />
            <Champ id="membre-depart-motif" libelle={t("personnel.depart_motif")} value={motif} onChange={(e) => setMotif(e.target.value)} />
            <Bouton variante="danger" onClick={() => partir.mutate()} disabled={partir.isPending || le === ""}>{t("personnel.partir")}</Bouton>
          </div>
        ) : (
          <div>
            <Bouton variante="secondaire" onClick={() => revenir.mutate()} disabled={revenir.isPending}>{t("personnel.revenir")}</Bouton>
          </div>
        )}
        <p className="text-sm text-muet">{t("personnel.depart_aide")}</p>
        <MessageErreur erreur={partir.error ?? revenir.error ?? enseigner.error} />
      </div>
    </details>
  );
}

function CategoriesEtFonctions({ categories, fonctions }: { categories: readonly CategoriePersonnel[]; fonctions: readonly FonctionPersonnel[] }) {
  const rafraichir = useRafraichir();
  const [nouvelle, setNouvelle] = useState("");
  const [libelles, setLibelles] = useState<Record<string, string>>({});
  const [fonction, setFonction] = useState("");
  const [categorieFonction, setCategorieFonction] = useState(categories.find((c) => c.active)?.id ?? "");
  const categorie = useMutation({
    mutationFn: (d: { id: string | null; libelle: string; active: boolean }) => appeler("categorie_personnel_enregistrer", d),
    onSuccess: () => {
      setNouvelle("");
      rafraichir();
    },
  });
  const enregistrerFonction = useMutation({
    mutationFn: (d: { id: string | null; categorie_id: string; libelle: string; active: boolean }) => appeler("fonction_personnel_enregistrer", d),
    onSuccess: () => {
      setFonction("");
      rafraichir();
    },
  });
  const libelle = (id: string, defaut: string) => libelles[id] ?? defaut;
  return (
    <details className="rounded border border-muet/30 p-3">
      <summary className="cursor-pointer text-sm font-semibold text-primaire">{t("personnel.categories_fonctions")}</summary>
      <div className="mt-3 flex flex-col gap-4">
        <p className="text-sm text-muet">{t("personnel.categories_aide")}</p>
        {categories.map((c) => (
          <div key={c.id} className="flex flex-col gap-2 rounded border border-muet/20 p-2">
            <div className="flex flex-wrap items-end gap-2">
              <Champ id={`categorie-${c.id}`} libelle={t("personnel.categorie")} value={libelle(c.id, c.libelle)} onChange={(e) => setLibelles((x) => ({ ...x, [c.id]: e.target.value }))} />
              <Bouton variante="secondaire" onClick={() => categorie.mutate({ id: c.id, libelle: libelle(c.id, c.libelle), active: c.active })}>{t("commun.enregistrer")}</Bouton>
              <Bouton variante="secondaire" onClick={() => categorie.mutate({ id: c.id, libelle: c.libelle, active: !c.active })}>{t(c.active ? "commun.retirer" : "personnel.remettre")}</Bouton>
              {c.active ? null : <span className="text-sm text-muet">{t("personnel.retiree")}</span>}
            </div>
            <ul className="flex flex-wrap gap-2">
              {fonctions
                .filter((f) => f.categorie_id === c.id)
                .map((f) => (
                  <li key={f.id} className={`flex items-center gap-1 rounded border px-2 py-1 text-sm ${f.active ? "border-muet/40 text-encre" : "border-muet/20 text-muet line-through"}`}>
                    <span>{f.libelle}</span>
                    <button
                      type="button"
                      className="text-primaire underline"
                      onClick={() => enregistrerFonction.mutate({ id: f.id, categorie_id: f.categorie_id, libelle: f.libelle, active: !f.active })}
                    >
                      {t(f.active ? "commun.retirer" : "personnel.remettre")}
                    </button>
                  </li>
                ))}
            </ul>
          </div>
        ))}
        <div className="flex flex-wrap items-end gap-2">
          <Champ id="categorie-nouvelle" libelle={t("personnel.nouvelle_categorie")} value={nouvelle} onChange={(e) => setNouvelle(e.target.value)} />
          <Bouton variante="secondaire" onClick={() => categorie.mutate({ id: null, libelle: nouvelle, active: true })} disabled={nouvelle.trim() === ""}>{t("commun.ajouter")}</Bouton>
        </div>
        <div className="flex flex-wrap items-end gap-2">
          <Selection id="fonction-categorie" libelle={t("personnel.categorie")} valeur={categorieFonction} surChangement={setCategorieFonction} options={categories.filter((c) => c.active).map((c) => ({ valeur: c.id, libelle: c.libelle }))} />
          <Champ id="fonction-nouvelle" libelle={t("personnel.nouvelle_fonction")} value={fonction} onChange={(e) => setFonction(e.target.value)} />
          <Bouton variante="secondaire" onClick={() => enregistrerFonction.mutate({ id: null, categorie_id: categorieFonction, libelle: fonction, active: true })} disabled={fonction.trim() === ""}>
            {t("commun.ajouter")}
          </Bouton>
        </div>
        <MessageErreur erreur={categorie.error ?? enregistrerFonction.error} />
      </div>
    </details>
  );
}
