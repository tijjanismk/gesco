import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { choisirLecons, enregistrerLeconsSous } from "../../api/fichiers";
import { commandeAutorisee } from "../../app/navigation";
import { Bouton } from "../../composants/Bouton";
import { MessageErreur, Section } from "../../composants/Messages";
import { date, t } from "../../i18n";
import type { ArriveeLecon, ImportAffiche, ImportFait, SelectionExport } from "../../types/gesco";

function libelleArrivee(a: ArriveeLecon): string {
  switch (a) {
    case "brouillon":
      return t("bibliotheque.echange.arrivee.brouillon");
    case "version_recue":
      return t("bibliotheque.echange.arrivee.version_recue");
    case "nouvelle":
      return t("bibliotheque.echange.arrivee.nouvelle");
    case "deja_la":
      return t("bibliotheque.echange.arrivee.deja_la");
    case "sans_place":
      return t("bibliotheque.echange.arrivee.sans_place");
  }
}

/** Le résumé d'un import, en une phrase par sorte d'arrivée ; rien d'ajouté se dit aussi. */
function resume(r: ImportFait): string[] {
  const lignes: string[] = [];
  if (r.brouillons > 0) lignes.push(t("bibliotheque.echange.resume.brouillons", { nombre: r.brouillons }));
  if (r.versions_recues > 0) lignes.push(t("bibliotheque.echange.resume.versions_recues", { nombre: r.versions_recues }));
  if (r.chapitres > 0) lignes.push(t("bibliotheque.echange.resume.chapitres", { nombre: r.chapitres }));
  if (r.ignorees > 0) lignes.push(t("bibliotheque.echange.resume.ignorees", { nombre: r.ignorees }));
  if (r.sans_place > 0) lignes.push(t("bibliotheque.echange.resume.sans_place", { nombre: r.sans_place }));
  if (r.brouillons + r.versions_recues === 0) lignes.unshift(t("bibliotheque.echange.resume.rien"));
  return lignes;
}

/** Une ligne de la liste des imports, avec « Publier les leçons reçues de cet import ». */
function LigneImport({ i, peutPublier }: { i: ImportAffiche; peutPublier: boolean }) {
  const client = useQueryClient();
  const [confirmer, setConfirmer] = useState(false);
  const publier = useMutation({
    mutationFn: () => appeler("bibliotheque_import_publier", { import_id: i.id }),
    onSuccess: (r) => {
      client.setQueryData(["bibliotheque-imports"], r.imports);
      setConfirmer(false);
      void client.invalidateQueries({ queryKey: ["bibliotheque"] });
    },
  });
  return (
    <li className="flex flex-col gap-1 border-b border-muet/30 py-2 text-sm text-encre">
      <p>
        {t("bibliotheque.echange.import_ligne", { fichier: i.nom_fichier, date: date(i.importe_le), qui: i.importe_par, nombre: i.lecons })}
        {i.ignorees > 0 ? ` ${t("bibliotheque.echange.import_ignorees", { nombre: i.ignorees })}` : ""}
      </p>
      {publier.data === undefined ? null : (
        <p role="status" className="text-vert">
          {t("bibliotheque.echange.publiees", { nombre: publier.data.publiees })}
        </p>
      )}
      <MessageErreur erreur={publier.error} />
      {!peutPublier || i.a_publier === 0 ? null : confirmer ? (
        <div className="flex flex-col gap-2">
          <p>{t("bibliotheque.echange.publier_confirmer", { nombre: i.a_publier })}</p>
          <div className="flex flex-wrap gap-2">
            <Bouton onClick={() => publier.mutate()} disabled={publier.isPending}>
              {t("bibliotheque.echange.publier")}
            </Bouton>
            <Bouton variante="secondaire" onClick={() => setConfirmer(false)}>
              {t("commun.annuler")}
            </Bouton>
          </div>
        </div>
      ) : (
        <div>
          <Bouton variante="secondaire" onClick={() => setConfirmer(true)}>
            {t("bibliotheque.echange.publier_import", { nombre: i.a_publier })}
          </Bouton>
        </div>
      )}
    </li>
  );
}

/** Réglages de la bibliothèque : importer des leçons d'une autre école (`library:manage`).
 *  Choix du fichier, vérification, liste de ce qui arrive et où cela se rangera, confirmation,
 *  résumé ; puis la liste des imports. Les leçons reçues arrivent en brouillons : rien n'est
 *  visible des élèves avant publication. */
export function ImportLecons() {
  const client = useQueryClient();
  const session = useQuery({ queryKey: ["session"], queryFn: () => appeler("session_courante", null) });
  const peutPublier = commandeAutorisee("bibliotheque_import_publier", session.data?.permissions ?? []);
  const imports = useQuery({ queryKey: ["bibliotheque-imports"], queryFn: () => appeler("bibliotheque_imports_lister", null) });
  const [chemin, setChemin] = useState<string | null>(null);
  const apercu = useMutation({ mutationFn: (c: string) => appeler("bibliotheque_import_apercu", { chemin: c }) });
  const importer = useMutation({
    mutationFn: (c: string) => appeler("bibliotheque_importer", { chemin: c }),
    onSuccess: (r) => {
      client.setQueryData(["bibliotheque-imports"], r.imports);
      setChemin(null);
      apercu.reset();
      void client.invalidateQueries({ queryKey: ["bibliotheque"] });
    },
  });
  const choisir = async () => {
    const c = await choisirLecons(t("bibliotheque.echange.filtre"));
    if (c === null) return;
    setChemin(c);
    importer.reset();
    apercu.mutate(c);
  };
  const a = apercu.data;
  const aRecevoir = a === undefined ? 0 : a.lecons.filter((l) => l.arrivee !== "deja_la" && l.arrivee !== "sans_place").length;
  return (
    <Section titre={t("bibliotheque.echange.titre")}>
      <p className="text-sm text-encre">{t("bibliotheque.echange.explication")}</p>
      <div>
        <Bouton variante="secondaire" onClick={() => void choisir()} disabled={apercu.isPending || importer.isPending}>
          {t("bibliotheque.echange.importer")}
        </Bouton>
      </div>
      <MessageErreur erreur={apercu.error} />
      {a === undefined || chemin === null ? null : (
        <div className="flex flex-col gap-2 text-sm text-encre">
          <p>{t("bibliotheque.echange.fichier", { fichier: a.nom_fichier, nombre: a.lecons.length })}</p>
          {a.deja_importe_le === null ? null : <p>{t("bibliotheque.echange.deja_importe", { date: date(a.deja_importe_le) })}</p>}
          <div className="overflow-x-auto">
            <table className="w-full border-collapse text-left">
              <thead>
                <tr className="text-muet">
                  <th scope="col" className="px-2 py-1">{t("bibliotheque.echange.colonne.lecon")}</th>
                  <th scope="col" className="px-2 py-1">{t("bibliotheque.echange.colonne.place")}</th>
                  <th scope="col" className="px-2 py-1">{t("bibliotheque.echange.colonne.arrivee")}</th>
                </tr>
              </thead>
              <tbody>
                {a.lecons.map((l, n) => (
                  <tr key={n} className="border-t border-muet/30 align-top">
                    <td className="px-2 py-1">{l.titre}</td>
                    <td className="px-2 py-1">
                      {t("bibliotheque.echange.place", { niveau: l.niveau, matiere: l.matiere, chapitre: l.chapitre })}
                      {l.chapitre_nouveau ? ` ${t("bibliotheque.echange.chapitre_nouveau")}` : ""}
                    </td>
                    <td className="px-2 py-1">{libelleArrivee(l.arrivee)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <div className="flex flex-wrap gap-2">
            <Bouton onClick={() => importer.mutate(chemin)} disabled={importer.isPending || aRecevoir === 0}>
              {t("bibliotheque.echange.confirmer", { nombre: aRecevoir })}
            </Bouton>
            <Bouton
              variante="secondaire"
              onClick={() => {
                setChemin(null);
                apercu.reset();
              }}
            >
              {t("commun.annuler")}
            </Bouton>
          </div>
        </div>
      )}
      <MessageErreur erreur={importer.error} />
      {importer.data === undefined ? null : (
        <div role="status" className="flex flex-col gap-1 text-sm text-vert">
          {resume(importer.data).map((ligne) => (
            <p key={ligne}>{ligne}</p>
          ))}
        </div>
      )}
      <MessageErreur erreur={imports.error} />
      {(imports.data ?? []).length === 0 ? null : (
        <>
          <h3 className="mt-2 font-semibold text-marine">{t("bibliotheque.echange.imports")}</h3>
          <ul className="flex flex-col">
            {(imports.data ?? []).map((i) => (
              <LigneImport key={i.id} i={i} peutPublier={peutPublier} />
            ))}
          </ul>
        </>
      )}
    </Section>
  );
}

/** Exporter un chapitre, une matière d'un niveau ou une sélection vers une autre école : la
 *  liste de ce qui partira, puis la boîte d'enregistrement du système. */
export function ExporterLecons({ selection, surFin }: { selection: SelectionExport; surFin: () => void }) {
  const apercu = useQuery({
    queryKey: ["bibliotheque", "export-apercu", selection],
    queryFn: () => appeler("bibliotheque_export_apercu", { selection }),
  });
  const exporter = useMutation({
    mutationFn: (chemin: string) => appeler("bibliotheque_exporter", { selection, chemin }),
  });
  const enregistrer = async () => {
    if (apercu.data === undefined) return;
    const chemin = await enregistrerLeconsSous(apercu.data.nom_propose, t("bibliotheque.echange.filtre"));
    if (chemin !== null) exporter.mutate(chemin);
  };
  const a = apercu.data;
  return (
    <div className="flex flex-col gap-2 rounded border border-muet/40 p-3 text-sm text-encre">
      <h3 className="font-semibold text-marine">{t("bibliotheque.echange.exporter_titre")}</h3>
      <MessageErreur erreur={apercu.error} />
      {a === undefined ? null : a.lecons.length === 0 ? (
        <p>{t("bibliotheque.echange.export_vide")}</p>
      ) : (
        <>
          <p>{t("bibliotheque.echange.export_liste", { nombre: a.lecons.length })}</p>
          <ul className="list-disc pl-5">
            {a.lecons.map((l) => (
              <li key={l.contenu_id}>
                {l.titre}
                <span className="text-muet"> · {l.chapitre}</span>
              </li>
            ))}
          </ul>
          <p className="text-muet">{t("bibliotheque.echange.export_rappel")}</p>
        </>
      )}
      <MessageErreur erreur={exporter.error} />
      {exporter.data === undefined ? null : (
        <p role="status" className="text-vert">
          {t("bibliotheque.echange.exporte", { nombre: exporter.data.lecons })}
        </p>
      )}
      <div className="flex flex-wrap gap-2">
        {a === undefined || a.lecons.length === 0 || exporter.data !== undefined ? null : (
          <Bouton onClick={() => void enregistrer()} disabled={exporter.isPending}>
            {t("bibliotheque.echange.enregistrer")}
          </Bouton>
        )}
        <Bouton variante="secondaire" onClick={surFin}>
          {exporter.data === undefined ? t("commun.annuler") : t("bulletins.fermer")}
        </Bouton>
      </div>
    </div>
  );
}
