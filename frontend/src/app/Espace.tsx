import { useMutation, useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";

import { appeler } from "../api";
import { Bouton } from "../composants/Bouton";
import { Selection } from "../composants/Champ";
import { MessageErreur } from "../composants/Messages";
import { t, texteDeCle } from "../i18n";
import { Accueil } from "../modules/accueil/Accueil";
import { Annees } from "../modules/administration/Annees";
import { Journal } from "../modules/administration/Journal";
import { Licence } from "../modules/administration/Licence";
import { Parametres } from "../modules/administration/Parametres";
import { Roles } from "../modules/administration/Roles";
import { Utilisateurs } from "../modules/administration/Utilisateurs";
import { MonCompte } from "../modules/compte/MonCompte";
import { Sauvegardes } from "../modules/sauvegarde/Sauvegardes";
import { Classes } from "../modules/scolarite/Classes";
import { Caisse, SessionsCaisse } from "../modules/finance/Caisse";
import { Impayes, JournalEncaissements } from "../modules/finance/Recouvrement";
import { Frais } from "../modules/finance/Frais";
import { Eleves } from "../modules/scolarite/Eleves";
import { Inscriptions } from "../modules/scolarite/Inscriptions";
import { Enseignants } from "../modules/pedagogie/Enseignants";
import { Absences } from "../modules/pedagogie/Absences";
import { Bulletins } from "../modules/pedagogie/Bulletins";
import { EmploiDuTemps } from "../modules/pedagogie/EmploiDuTemps";
import { FinAnnee } from "../modules/scolarite/FinAnnee";
import { Cantine } from "../modules/finance/Cantine";
import { Depenses } from "../modules/finance/Depenses";
import { Fournisseurs } from "../modules/finance/Fournisseurs";
import { JournalTresorerie } from "../modules/finance/Journal";
import { Livres } from "../modules/finance/Livres";
import { Tresorerie } from "../modules/finance/Tresorerie";
import { Notes } from "../modules/pedagogie/Notes";
import { Pedagogie } from "../modules/pedagogie/Pedagogie";
import { Niveaux } from "../modules/scolarite/Niveaux";
import { Conges } from "../modules/rh/Conges";
import { Heures } from "../modules/rh/Heures";
import { Personnel } from "../modules/rh/Personnel";
import { Paie } from "../modules/paie/Paie";
import { commandeAutorisee, GROUPES_REPLIES, type IdSection, sectionsAutorisees } from "./navigation";
import { RechercheGlobale } from "./RechercheGlobale";
import { useSession } from "./session";

type Theme = "systeme" | "clair" | "sombre";

function appliquerTheme(theme: Theme) {
  const racine = document.documentElement;
  racine.classList.toggle("clair", theme === "clair");
  racine.classList.toggle("sombre", theme === "sombre");
}

/** Verrouille l'écran après l'inactivité fixée par le noyau (§6.2). */
function useVerrouillageInactivite(delai: number | undefined, verrouiller: () => void) {
  useEffect(() => {
    if (delai === undefined) return;
    let minuterie = window.setTimeout(verrouiller, delai);
    const relancer = () => {
      window.clearTimeout(minuterie);
      minuterie = window.setTimeout(verrouiller, delai);
    };
    const evenements = ["keydown", "pointerdown", "pointermove", "wheel"] as const;
    evenements.forEach((e) => window.addEventListener(e, relancer, { passive: true }));
    return () => {
      window.clearTimeout(minuterie);
      evenements.forEach((e) => window.removeEventListener(e, relancer));
    };
  }, [delai, verrouiller]);
}

export function Espace({ etablissement }: { etablissement: string }) {
  const session = useSession();
  const courante = useQuery({ queryKey: ["session"], queryFn: () => appeler("session_courante", null) });
  const avertissements = useQuery({ queryKey: ["avertissements"], queryFn: () => appeler("avertissements", null) });
  const [section, setSection] = useState<IdSection>("accueil");
  const [deplies, setDeplies] = useState<readonly string[]>([]);
  const [eleveOuvert, setEleveOuvert] = useState<string | null>(null);
  // Chaque clic du menu rouvre l'écran à son début, même s'il est déjà affiché (une grille de
  // notes ouverte revient à la liste des évaluations).
  const [visite, setVisite] = useState(0);
  const [recherche, setRecherche] = useState(false);
  useEffect(() => {
    const raccourci = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setRecherche(true);
      }
    };
    window.addEventListener("keydown", raccourci);
    return () => window.removeEventListener("keydown", raccourci);
  }, []);
  const [theme, setTheme] = useState<Theme>("systeme");
  const verrou = useMutation({ mutationFn: () => appeler("verrouiller", null), onSettled: session.verrouiller });
  const sortie = useMutation({ mutationFn: () => appeler("deconnexion", null), onSettled: session.fermer });

  useVerrouillageInactivite(courante.data?.verrouillage_inactivite_ms, verrou.mutate);

  if (courante.isPending) return <p className="p-6 text-muet">{t("commun.chargement")}</p>;
  if (courante.isError) return <div className="p-6"><MessageErreur erreur={courante.error} /></div>;
  const { session: ouverte, permissions, doit_changer_mot_de_passe } = courante.data;
  const sections = sectionsAutorisees(permissions);
  const peutChercher = commandeAutorisee("eleves_rechercher", permissions) && !doit_changer_mot_de_passe;
  const groupes = [...new Set(sections.map((s) => s.groupe))];

  return (
    <div className="flex h-screen">
      <nav className="flex w-56 shrink-0 flex-col gap-4 overflow-y-auto bg-marine p-4 text-sur-marine">
        <p className="text-lg font-semibold">{t("app.nom")}</p>
        {groupes.map((groupe) => {
          const repliable = GROUPES_REPLIES.includes(groupe);
          const ouvert = !repliable || deplies.includes(groupe) || sections.some((s) => s.groupe === groupe && s.id === section);
          return (
          <div key={groupe} className="flex flex-col gap-1">
            {repliable ? (
              <button
                type="button"
                aria-expanded={ouvert}
                onClick={() => setDeplies((d) => (d.includes(groupe) ? d.filter((g) => g !== groupe) : [...d, groupe]))}
                className="flex items-center justify-between rounded text-left text-xs text-sur-marine/70 hover:text-sur-marine"
              >
                <span>{t(groupe)}</span>
                <span aria-hidden="true">{ouvert ? "▾" : "▸"}</span>
              </button>
            ) : (
              <p className="text-xs text-sur-marine/70">{t(groupe)}</p>
            )}
            {ouvert
              ? sections
              .filter((s) => s.groupe === groupe)
              .map((s) => (
                <button
                  key={s.id}
                  type="button"
                  disabled={doit_changer_mot_de_passe}
                  onClick={() => {
                    setSection(s.id);
                    setEleveOuvert(null);
                    setVisite((v) => v + 1);
                  }}
                  className={`rounded px-2 py-1 text-left text-sm ${section === s.id ? "bg-sur-marine/15 font-medium" : "hover:bg-sur-marine/10"}`}
                >
                  {t(s.libelle)}
                </button>
              ))
              : null}
          </div>
          );
        })}
      </nav>
      <div className="flex flex-1 flex-col overflow-hidden">
        <header className="flex flex-wrap items-center gap-4 border-b border-muet/30 bg-surface px-4 py-2">
          <p className="text-sm text-muet">
            {etablissement}
            {courante.data.annee_courante === null ? null : ` · ${t("bandeau.annee", { annee: courante.data.annee_courante })}`}
          </p>
          <p className="text-base font-semibold text-encre">{t("bandeau.utilisateur", { nom: ouverte.nom_affiche })}</p>
          <div className="ml-auto flex items-center gap-2">
            {peutChercher ? <Bouton variante="secondaire" onClick={() => setRecherche(true)}>{t("recherche.ouvrir")}</Bouton> : null}
            <Selection
              id="theme"
              libelle={t("bandeau.theme")}
              valeur={theme}
              surChangement={(v) => {
                const choisi = v === "clair" || v === "sombre" ? v : "systeme";
                setTheme(choisi);
                appliquerTheme(choisi);
              }}
              options={[
                { valeur: "systeme", libelle: t("bandeau.theme.systeme") },
                { valeur: "clair", libelle: t("bandeau.theme.clair") },
                { valeur: "sombre", libelle: t("bandeau.theme.sombre") },
              ]}
            />
            <Bouton variante="secondaire" onClick={() => verrou.mutate()}>{t("bandeau.verrouiller")}</Bouton>
            <Bouton variante="secondaire" onClick={() => sortie.mutate()}>{t("bandeau.deconnexion")}</Bouton>
          </div>
        </header>
        {courante.data.pin_defini || doit_changer_mot_de_passe ? null : (
          <div className="flex flex-wrap items-center gap-3 border-b border-ocre bg-surface px-4 py-2">
            <p className="text-sm text-ocre">{t("compte.pin_absent")}</p>
            <Bouton variante="secondaire" onClick={() => setSection("compte")}>{t("compte.definir_pin")}</Bouton>
          </div>
        )}
        {avertissements.data === undefined || avertissements.data.length === 0 ? null : (
          <ul className="flex flex-col gap-1 border-b border-ocre bg-surface px-4 py-2">
            {avertissements.data.map((a) => (
              <li key={a.cle} className="text-sm text-ocre">{texteDeCle(a.cle)}</li>
            ))}
          </ul>
        )}
        <main className="flex-1 overflow-y-auto p-4">
          {doit_changer_mot_de_passe ? (
            <MonCompte changementExige />
          ) : (
            <Contenu
              key={`${section}-${visite}`}
              section={section}
              nom={ouverte.nom_affiche}
              eleveOuvert={eleveOuvert}
              surOuvrirEleve={(id) => {
                setEleveOuvert(id);
                setSection("eleves");
              }}
            />
          )}
        </main>
        {recherche && peutChercher ? (
          <RechercheGlobale
            surFermer={() => setRecherche(false)}
            surOuvrir={(id) => {
              setRecherche(false);
              setEleveOuvert(id);
              setSection("eleves");
            }}
          />
        ) : null}
      </div>
    </div>
  );
}

function Contenu({
  section,
  nom,
  eleveOuvert,
  surOuvrirEleve,
}: {
  section: IdSection;
  nom: string;
  eleveOuvert: string | null;
  surOuvrirEleve: (id: string) => void;
}) {
  switch (section) {
    case "accueil":
      return <Accueil nom={nom} />;
    case "compte":
      return <MonCompte changementExige={false} />;
    case "utilisateurs":
      return <Utilisateurs />;
    case "roles":
      return <Roles />;
    case "parametres":
      return <Parametres />;
    case "annees":
      return <Annees />;
    case "niveaux":
      return <Niveaux />;
    case "pedagogie":
      return <Pedagogie />;
    case "enseignants":
      return <Enseignants />;
    case "personnel":
      return <Personnel />;
    case "heures":
      return <Heures />;
    case "conges":
      return <Conges />;
    case "paie":
      return <Paie />;
    case "notes":
      return <Notes />;
    case "absences":
      return <Absences />;
    case "bulletins":
      return <Bulletins />;
    case "fin-annee":
      return <FinAnnee />;
    case "emploi-du-temps":
      return <EmploiDuTemps />;
    case "depenses":
      return <Depenses />;
    case "fournisseurs":
      return <Fournisseurs />;
    case "livres":
      return <Livres />;
    case "journal-tresorerie":
      return <JournalTresorerie />;
    case "cantine":
      return <Cantine />;
    case "tresorerie":
      return <Tresorerie />;
    case "classes":
      return <Classes />;
    case "frais":
      return <Frais />;
    case "caisse":
      return <Caisse />;
    case "sessions-caisse":
      return <SessionsCaisse />;
    case "impayes":
      return <Impayes surOuvrirEleve={surOuvrirEleve} />;
    case "journal-encaissements":
      return <JournalEncaissements />;
    case "inscriptions":
      return <Inscriptions surOuvrirEleve={surOuvrirEleve} />;
    case "eleves":
      return <Eleves key={eleveOuvert ?? "liste"} ouvert={eleveOuvert} />;
    case "sauvegarde":
      return <Sauvegardes />;
    case "journal":
      return <Journal />;
    case "licence":
      return <Licence />;
  }
}
