import { useQuery } from "@tanstack/react-query";

import { appeler } from "../api";
import type { ModuleEcole } from "../types/gesco";

/** Modules que l'école a allumés, d'après la session courante : leurs écrans, leurs boutons et
 *  leurs réglages ne paraissent qu'alors. La garde Rust refuse de toute façon ce qui exige un
 *  module éteint. */
export function useModules(): readonly ModuleEcole[] {
  const session = useQuery({ queryKey: ["session"], queryFn: () => appeler("session_courante", null) });
  return session.data?.modules ?? [];
}

/** Vrai si l'école a allumé les messages aux familles. */
export function useSmsActif(): boolean {
  return useModules().includes("sms");
}
