import { fileURLToPath } from "node:url";

import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// Écrans des élèves (portail sur le Wi-Fi de l'école) : construits à part du bureau, sans rien de
// Tauri, servis par le poste principal. Tous les fichiers vont dans `assets/` (une seule route
// les sert) ; les vieux téléphones Android lisent l'ES2020.
export default defineConfig({
  root: fileURLToPath(new URL(".", import.meta.url)),
  base: "/",
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: { port: 1421, strictPort: true, proxy: { "/eleve/api": "http://localhost:47801" } },
  build: {
    target: "es2020",
    // Le dossier livré par l'installeur ; `compresser.mjs --vider` le vide avant.
    outDir: fileURLToPath(new URL("../../crates/gesco-app/ressources/portail-eleve", import.meta.url)),
    emptyOutDir: false,
    assetsDir: "assets",
    chunkSizeWarningLimit: 400,
  },
});
