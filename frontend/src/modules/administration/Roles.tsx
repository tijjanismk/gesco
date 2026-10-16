import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { Bouton } from "../../composants/Bouton";
import { CaseACocher, Champ, Formulaire, Selection } from "../../composants/Champ";
import { MessageEnregistre, MessageErreur, Section } from "../../composants/Messages";
import { Tableau } from "../../composants/Tableau";
import { type Cle, libellePermission, t } from "../../i18n";
import type { FicheRole } from "../../types/gesco";

/** Domaines de l'écran (SPEC_PHASE_1 §5.1), par préfixe de code de permission. */
export const DOMAINES: readonly { cle: Cle; prefixes: readonly string[] }[] = [
  { cle: "roles.domaine.administration", prefixes: ["core"] },
  { cle: "roles.domaine.annee", prefixes: ["academic_year", "periods"] },
  { cle: "roles.domaine.eleves", prefixes: ["students", "enrollments"] },
  { cle: "roles.domaine.notes", prefixes: ["grades", "reports"] },
  { cle: "roles.domaine.vie_scolaire", prefixes: ["attendance", "sanctions", "convocations", "timetable", "canteen"] },
  { cle: "roles.domaine.finance", prefixes: ["fees", "invoices", "payments", "cashbox", "expenses", "treasury"] },
  { cle: "roles.domaine.personnel", prefixes: ["employees", "contracts", "leave", "payroll"] },
  { cle: "roles.domaine.documents", prefixes: ["documents", "templates", "notifications"] },
  { cle: "roles.domaine.donnees", prefixes: ["import", "export", "licence"] },
];

const ADMINISTRATEUR = "administrateur";

function domaine(code: string): string {
  return code.split(":")[0] ?? code;
}

/** Rôles et permissions (SPEC_PHASE_1 §5) : la garde Rust refuse tout cumul notes et caisse
 *  et toute permission que l'auteur ne détient pas ; l'écran ne fait que présenter. */
export function Roles() {
  const roles = useQuery({ queryKey: ["roles"], queryFn: () => appeler("roles_lister", null) });
  const catalogue = useQuery({ queryKey: ["permissions"], queryFn: () => appeler("permissions_catalogue", null) });
  const [choisi, setChoisi] = useState<string | null>(null);
  const fiche = roles.data?.find((r) => r.role.code === choisi);
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("roles.titre")}</h1>
      <MessageErreur erreur={roles.error ?? catalogue.error} />
      <div className="flex flex-wrap items-start gap-4">
        <div className="min-w-0 flex-1">
          <Tableau
            nomExport="roles"
            titre={t("roles.titre")}
            lignes={roles.data ?? []}
            cle={(r) => r.role.id}
            surSelection={(r) => setChoisi(r.role.code)}
            colonnes={[
              { titre: t("roles.nom"), valeur: (r) => r.role.libelle },
              { titre: t("roles.type"), valeur: (r) => (r.role.systeme ? t("roles.livre") : t("roles.personnalise")) },
              { titre: t("roles.nombre"), valeur: (r) => String(r.permissions.length) },
            ]}
          />
        </div>
        <aside className="flex w-96 flex-col gap-4">
          <NouveauRole roles={roles.data ?? []} surCree={setChoisi} />
        </aside>
      </div>
      {fiche === undefined || catalogue.data === undefined ? null : (
        <EditeurRole
          key={`${fiche.role.code}-${fiche.permissions.join(",")}-${fiche.role.libelle}`}
          fiche={fiche}
          catalogue={catalogue.data}
          surSupprime={() => setChoisi(null)}
        />
      )}
    </div>
  );
}

function NouveauRole({ roles, surCree }: { roles: readonly FicheRole[]; surCree: (code: string) => void }) {
  const client = useQueryClient();
  const [libelle, setLibelle] = useState("");
  const [modele, setModele] = useState("");
  const creation = useMutation({
    mutationFn: () =>
      appeler("role_creer", {
        libelle,
        permissions: roles.find((r) => r.role.code === modele)?.permissions ?? [],
      }),
    onSuccess: (cree) => {
      setLibelle("");
      setModele("");
      void client.invalidateQueries({ queryKey: ["roles"] });
      surCree(cree.role.code);
    },
  });
  return (
    <Section titre={t("roles.nouveau")}>
      <Formulaire surEnvoi={() => creation.mutate()}>
        <Champ id="nouveau-role-nom" libelle={t("roles.nom")} value={libelle} onChange={(e) => setLibelle(e.target.value)} />
        <Selection
          id="nouveau-role-modele"
          libelle={t("roles.partir_de")}
          valeur={modele}
          surChangement={setModele}
          options={[
            { valeur: "", libelle: t("roles.vide") },
            ...roles.filter((r) => r.role.code !== ADMINISTRATEUR).map((r) => ({ valeur: r.role.code, libelle: r.role.libelle })),
          ]}
        />
        <MessageErreur erreur={creation.error} />
        <div><Bouton type="submit" disabled={creation.isPending}>{t("roles.creer")}</Bouton></div>
      </Formulaire>
    </Section>
  );
}

function EditeurRole({
  fiche,
  catalogue,
  surSupprime,
}: {
  fiche: FicheRole;
  catalogue: readonly string[];
  surSupprime: () => void;
}) {
  const client = useQueryClient();
  const { role } = fiche;
  const intouchable = role.code === ADMINISTRATEUR;
  const [libelle, setLibelle] = useState(role.libelle);
  const [cochees, setCochees] = useState<readonly string[]>(fiche.permissions);
  const [enregistre, setEnregistre] = useState<number | null>(null);
  const rafraichir = () => {
    setEnregistre(Date.now());
    void client.invalidateQueries({ queryKey: ["roles"] });
  };
  const modification = useMutation({
    mutationFn: () => appeler("role_modifier", { code: role.code, libelle, permissions: [...cochees] }),
    onSuccess: rafraichir,
  });
  const retablissement = useMutation({ mutationFn: () => appeler("role_retablir", { code: role.code }), onSuccess: rafraichir });
  const suppression = useMutation({
    mutationFn: () => appeler("role_supprimer", { code: role.code }),
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: ["roles"] });
      surSupprime();
    },
  });
  const basculer = (code: string, coche: boolean) =>
    setCochees(coche ? [...cochees, code] : cochees.filter((c) => c !== code));
  return (
    <Section titre={t("roles.composition", { nom: role.libelle })}>
      {intouchable ? <p className="text-sm text-muet">{t("roles.administrateur_note")}</p> : null}
      <Formulaire surEnvoi={() => modification.mutate()}>
        <Champ
          id="role-nom"
          libelle={t("roles.nom")}
          value={libelle}
          disabled={role.systeme}
          aide={role.systeme ? t("roles.nom_livre") : undefined}
          onChange={(e) => setLibelle(e.target.value)}
        />
        <div className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-3">
          {DOMAINES.map((d) => (
            <fieldset key={d.cle} className="flex flex-col gap-1 rounded border border-muet/30 p-3">
              <legend className="px-1 text-sm font-medium text-encre">{t(d.cle)}</legend>
              {catalogue
                .filter((code) => d.prefixes.includes(domaine(code)))
                .map((code) => (
                  <CaseACocher
                    key={code}
                    id={`permission-${code}`}
                    libelle={libellePermission(code)}
                    coche={cochees.includes(code)}
                    desactive={intouchable}
                    surChangement={(coche) => basculer(code, coche)}
                  />
                ))}
            </fieldset>
          ))}
        </div>
        <MessageErreur erreur={modification.error ?? retablissement.error ?? suppression.error} />
        <MessageEnregistre instant={enregistre} />
        {intouchable ? null : (
          <div className="flex flex-wrap gap-2">
            <Bouton type="submit" disabled={modification.isPending}>{t("commun.enregistrer")}</Bouton>
            {role.systeme ? (
              <Bouton variante="secondaire" onClick={() => retablissement.mutate()} disabled={retablissement.isPending}>
                {t("roles.retablir")}
              </Bouton>
            ) : (
              <Bouton variante="danger" onClick={() => suppression.mutate()} disabled={suppression.isPending}>
                {t("roles.supprimer")}
              </Bouton>
            )}
          </div>
        )}
      </Formulaire>
    </Section>
  );
}
