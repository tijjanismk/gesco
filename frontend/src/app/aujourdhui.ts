import type { SessionCourante } from "../types/gesco";
import { clientRequetes } from "./requetes";

/** Jour courant `AAAA-MM-JJ` selon l'horloge de GESCO, que donne la session (R-03) ; celle du
 *  poste ne sert qu'avant la connexion. */
export function aujourdHui(): string {
  const session = clientRequetes.getQueryData<SessionCourante>(["session"]);
  return session?.aujourd_hui ?? new Date().toISOString().slice(0, 10);
}
