import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { appeler } from "../../api";
import { type EtatPareFeu, lirePareFeu, ouvrirPareFeu } from "../../api/poste";
import { Bouton } from "../../composants/Bouton";
import { MessageErreur, Section } from "../../composants/Messages";
import { type Cle, date, dateHeure, heure, t } from "../../i18n";
import type { EtatReseau, PosteRelie, PostesCouverts } from "../../types/gesco";

/** Ce qu'on demande de confirmer avant d'agir : éteindre, renouveler la clé, ou révoquer un poste
 *  (son identifiant). */
type AConfirmer = "eteindre" | "cle" | { poste: string } | null;

function nombreDePostes(nombre: number): string {
  if (nombre === 0) return t("postes.nombre.aucun");
  return nombre === 1 ? t("postes.nombre.un") : t("postes.nombre.plusieurs", { nombre });
}

function couverts(c: PostesCouverts): string {
  return c.type === "illimites" ? t("postes.couverts.illimites") : nombreDePostes(c.nombre);
}

/** Postes de l'école : le poste principal allume le réseau, relie les autres ordinateurs par un
 *  code, les révoque, renouvelle la clé du réseau. */
export function Postes() {
  const client = useQueryClient();
  // Relu toutes les dix secondes : un poste qui vient de se relier ou d'ouvrir une session paraît
  // sans recharger l'écran, et un code expiré disparaît.
  const etat = useQuery({
    queryKey: ["reseau"],
    queryFn: () => appeler("reseau_etat_lire", null),
    refetchInterval: 10_000,
  });
  const [aConfirmer, setAConfirmer] = useState<AConfirmer>(null);
  const action = useMutation({
    mutationFn: (appel: () => Promise<EtatReseau>) => appel(),
    onSuccess: (nouvel) => client.setQueryData(["reseau"], nouvel),
    onSettled: () => setAConfirmer(null),
  });
  const reseau = etat.data;
  const port = reseau?.port;
  // Le pare-feu de Windows ne se lit que réseau allumé, à l'ouverture de l'écran et après chaque
  // geste, pas toutes les dix secondes : la lecture lance des programmes de Windows.
  const pareFeu = useQuery({
    queryKey: ["pare-feu", port],
    queryFn: () => lirePareFeu(port ?? 0),
    enabled: port !== undefined && reseau?.actif === true,
    staleTime: Infinity,
  });
  const ouverture = useMutation({
    mutationFn: (p: number) => ouvrirPareFeu(p),
    onSuccess: (nouvel, p) => client.setQueryData(["pare-feu", p], nouvel),
  });
  // Le pare-feu s'ouvre avant que GESCO n'écoute : Windows n'a pas à poser sa propre question.
  const activer = async (p: number) => {
    await ouverture.mutateAsync(p);
    return appeler("reseau_activer", null);
  };
  return (
    <div className="flex max-w-4xl flex-col gap-4">
      <h1 className="text-xl font-semibold text-encre">{t("postes.titre")}</h1>
      <p className="text-sm text-muet">{t("postes.presentation")}</p>
      <MessageErreur erreur={etat.error ?? action.error ?? ouverture.error ?? pareFeu.error} />
      {reseau === undefined ? null : (
        <>
          <Section titre={t("postes.reseau")}>
            <p className={`text-sm font-medium ${reseau.actif ? "text-vert" : "text-encre"}`}>
              {t(reseau.actif ? "postes.reseau.allume" : "postes.reseau.eteint")}
            </p>
            {reseau.actif ? <Reperes reseau={reseau} /> : null}
            {reseau.actif && pareFeu.data !== undefined ? (
              <PareFeu
                etat={pareFeu.data}
                enCours={ouverture.isPending || pareFeu.isFetching}
                surOuvrir={() => ouverture.mutate(reseau.port)}
                surVerifier={() => void pareFeu.refetch()}
              />
            ) : null}
            <div className="flex flex-wrap items-center gap-2">
              {!reseau.actif ? (
                <Bouton disabled={action.isPending} onClick={() => action.mutate(() => activer(reseau.port))}>
                  {t("postes.activer")}
                </Bouton>
              ) : aConfirmer === "eteindre" ? (
                <>
                  <Bouton variante="danger" disabled={action.isPending} onClick={() => action.mutate(() => appeler("reseau_eteindre", null))}>
                    {t("postes.eteindre_confirmer")}
                  </Bouton>
                  <Bouton variante="secondaire" onClick={() => setAConfirmer(null)}>{t("commun.annuler")}</Bouton>
                </>
              ) : (
                <Bouton variante="secondaire" onClick={() => setAConfirmer("eteindre")}>{t("postes.eteindre")}</Bouton>
              )}
            </div>
            <p className="text-sm text-muet">{t(reseau.actif ? "postes.eteindre_aide" : "postes.activer_aide")}</p>
          </Section>

          {reseau.actif ? (
            <Appairage
              reseau={reseau}
              enCours={action.isPending}
              surOuvrir={() => action.mutate(() => appeler("poste_appairage_ouvrir", null))}
              surFermer={() => action.mutate(() => appeler("poste_appairage_fermer", null))}
            />
          ) : null}

          <Section titre={t("postes.liste")}>
            {reseau.postes.length === 0 ? (
              <p className="text-sm text-muet">{t("postes.liste_vide")}</p>
            ) : (
              <div className="overflow-x-auto">
                <table className="w-full border-collapse font-condensee text-sm tabular-nums">
                  <thead>
                    <tr>
                      {(["postes.colonne.nom", "postes.colonne.relie_le", "postes.colonne.derniere", "postes.colonne.etat"] as const).map((c) => (
                        <th key={c} scope="col" className="border-b border-muet px-2 py-1 text-left font-medium text-muet">{t(c)}</th>
                      ))}
                      <th scope="col" className="border-b border-muet px-2 py-1"><span className="sr-only">{t("postes.colonne.action")}</span></th>
                    </tr>
                  </thead>
                  <tbody>
                    {reseau.postes.map((p) => (
                      <LignePoste
                        key={p.id}
                        poste={p}
                        connecte={reseau.connectes.includes(p.id)}
                        aConfirmer={typeof aConfirmer === "object" && aConfirmer?.poste === p.id}
                        enCours={action.isPending}
                        surDemander={() => setAConfirmer({ poste: p.id })}
                        surAnnuler={() => setAConfirmer(null)}
                        surRevoquer={() => action.mutate(() => appeler("poste_revoquer", { poste_id: p.id }))}
                      />
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </Section>

          {reseau.empreinte_courte === null ? null : (
            <Section titre={t("postes.cle")}>
              <p className="text-sm text-muet">{t("postes.cle_aide")}</p>
              <div className="flex flex-wrap items-center gap-2">
                {aConfirmer === "cle" ? (
                  <>
                    <Bouton variante="danger" disabled={action.isPending} onClick={() => action.mutate(() => appeler("reseau_cle_renouveler", null))}>
                      {t("postes.cle_confirmer")}
                    </Bouton>
                    <Bouton variante="secondaire" onClick={() => setAConfirmer(null)}>{t("commun.annuler")}</Bouton>
                  </>
                ) : (
                  <Bouton variante="secondaire" onClick={() => setAConfirmer("cle")}>{t("postes.cle_renouveler")}</Bouton>
                )}
              </div>
            </Section>
          )}
        </>
      )}
    </div>
  );
}

/** Ce qu'on lit sur le poste principal pour relier un poste : son nom, son adresse, son empreinte,
 *  et ce que la licence couvre. */
function Reperes({ reseau }: { reseau: EtatReseau }) {
  const lignes: [Cle, string][] = [
    ["postes.nom_principal", reseau.nom_poste_principal],
    ["postes.adresse", reseau.adresses.length === 0 ? t("postes.adresse_aucune") : reseau.adresses.join(" · ")],
    ["postes.port", String(reseau.port)],
    ["postes.empreinte", reseau.empreinte_courte ?? t("postes.empreinte_attente")],
    ["postes.couverts", couverts(reseau.couverts)],
    ["postes.en_service", nombreDePostes(reseau.en_service)],
  ];
  return (
    <dl className="grid grid-cols-2 gap-x-4 gap-y-1 text-sm">
      {lignes.map(([cle, valeur]) => (
        <div key={cle} className="contents">
          <dt className="text-muet">{t(cle)}</dt>
          <dd className="font-condensee text-encre">{valeur}</dd>
        </div>
      ))}
    </dl>
  );
}

/** Pare-feu de Windows : ouvert aux postes reliés ou non, et ce qu'il reste à faire. */
function PareFeu({ etat, enCours, surOuvrir, surVerifier }: {
  etat: EtatPareFeu;
  enCours: boolean;
  surOuvrir: () => void;
  surVerifier: () => void;
}) {
  return (
    <>
      {etat.ouvert ? (
        <p className="text-sm text-encre">{t("postes.pare_feu.ouvert")}</p>
      ) : (
        <div role="alert" className="flex flex-col items-start gap-2 rounded border border-ocre p-3 text-sm text-ocre">
          <p>{t("postes.pare_feu.ferme")}</p>
          <Bouton disabled={enCours} onClick={surOuvrir}>{t("postes.pare_feu.ouvrir")}</Bouton>
        </div>
      )}
      {etat.reseau_public ? (
        <div className="flex flex-col items-start gap-2 rounded border border-ocre p-3 text-sm text-ocre">
          <p>{t("postes.pare_feu.reseau_public")}</p>
          <Bouton variante="secondaire" disabled={enCours} onClick={surVerifier}>{t("postes.pare_feu.verifier")}</Bouton>
        </div>
      ) : null}
    </>
  );
}

function Appairage({ reseau, enCours, surOuvrir, surFermer }: {
  reseau: EtatReseau;
  enCours: boolean;
  surOuvrir: () => void;
  surFermer: () => void;
}) {
  const complet = reseau.couverts.type === "nombre" && reseau.en_service >= reseau.couverts.nombre;
  const code = reseau.appairage;
  return (
    <Section titre={t("postes.relier")}>
      {code === null ? (
        <>
          <p className="text-sm text-muet">{t(complet ? "postes.complet" : "postes.relier_aide")}</p>
          <div>
            <Bouton disabled={enCours || complet} onClick={surOuvrir}>{t("postes.code_demander")}</Bouton>
          </div>
        </>
      ) : (
        <>
          <p className="text-sm text-muet">{t("postes.code")}</p>
          <p role="status" className="select-all font-condensee text-4xl font-semibold tracking-widest text-encre">{code.code}</p>
          <p className="text-sm text-encre">{t("postes.code_validite", { heure: heure(code.expire_le) })}</p>
          {reseau.empreinte_courte === null ? null : (
            <p className="text-sm text-encre">{t("postes.code_comparer", { empreinte: reseau.empreinte_courte })}</p>
          )}
          <div>
            <Bouton variante="secondaire" disabled={enCours} onClick={surFermer}>{t("postes.code_annuler")}</Bouton>
          </div>
        </>
      )}
    </Section>
  );
}

function LignePoste({ poste, connecte, aConfirmer, enCours, surDemander, surAnnuler, surRevoquer }: {
  poste: PosteRelie;
  connecte: boolean;
  aConfirmer: boolean;
  enCours: boolean;
  surDemander: () => void;
  surAnnuler: () => void;
  surRevoquer: () => void;
}) {
  const revoque = poste.revoque_le !== null;
  const situation = poste.revoque_le !== null
    ? t("postes.etat.revoque", { date: date(poste.revoque_le) })
    : t(connecte ? "postes.etat.connecte" : "postes.etat.en_service");
  const cellule = "border-b border-muet/30 px-2 py-1";
  return (
    <tr className={revoque ? "text-muet" : "text-encre"}>
      <th scope="row" className={`${cellule} text-left font-medium`}>{poste.nom}</th>
      <td className={cellule}>{dateHeure(poste.relie_le)}</td>
      <td className={cellule}>{poste.derniere_connexion === null ? t("postes.jamais") : dateHeure(poste.derniere_connexion)}</td>
      <td className={`${cellule} ${connecte ? "font-medium text-vert" : ""}`}>{situation}</td>
      <td className={`${cellule} text-right`}>
        {revoque ? null : aConfirmer ? (
          <span className="inline-flex flex-wrap justify-end gap-2">
            <Bouton variante="danger" disabled={enCours} onClick={surRevoquer}>{t("postes.revoquer_confirmer", { nom: poste.nom })}</Bouton>
            <Bouton variante="secondaire" onClick={surAnnuler}>{t("commun.annuler")}</Bouton>
          </span>
        ) : (
          <Bouton variante="secondaire" aria-label={t("postes.revoquer_nom", { nom: poste.nom })} onClick={surDemander}>{t("postes.revoquer")}</Bouton>
        )}
      </td>
    </tr>
  );
}
