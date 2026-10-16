import { defineConfig } from "@playwright/test";

// Essais de bout en bout sur l'application Tauri (`e2e/`) : `gesco-app` compilé en
// développement charge le frontend servi par Vite, et Playwright le pilote par le port de
// débogage de WebView2. Aucun navigateur Playwright n'est téléchargé.
export default defineConfig({
  testDir: "e2e",
  timeout: 120_000,
  workers: 1,
  reporter: "list",
  webServer: { command: "pnpm dev", port: 1420, reuseExistingServer: true },
});
