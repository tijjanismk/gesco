/** Moyen d'atteindre la frontière Rust : commande Tauri locale ou HTTP (ADR-014, R-38). */
export interface Transport {
  appeler(nom: string, jeton: string | null, entree: unknown): Promise<unknown>;
}
