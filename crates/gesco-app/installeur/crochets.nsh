; Crochets de l'installeur de GESCO (modele NSIS de Tauri). Textes sans accents : ce fichier est
; lu tel quel par makensis.
;
; Desinstallation : la page de confirmation de Tauri porte la case « Supprimer les donnees de
; l'application », qui efface le dossier des donnees (%APPDATA%\ml.gesco.poste) : la base de
; l'ecole et ses cles. Quand ce dossier contient la base d'une ecole,
; la case est ignoree et la personne en est prevenue. Un poste relie n'a pas de base : son
; dossier (reglage du poste, journal technique) s'efface si la case est cochee.
;
; Si Tauri renomme $DeleteAppDataCheckboxState ou $UpdateMode, makensis echoue a la
; construction de l'installeur : rien ne passe en silence.
!macro NSIS_HOOK_PREUNINSTALL
  ${If} $DeleteAppDataCheckboxState = 1
  ${AndIf} $UpdateMode <> 1
    ${If} ${FileExists} "$APPDATA\${BUNDLEID}\gesco.db"
      StrCpy $DeleteAppDataCheckboxState 0
      MessageBox MB_OK|MB_ICONINFORMATION "Les donnees de l'ecole restent sur cet ordinateur : GESCO ne les efface pas en se desinstallant.$\r$\n$\r$\nElles sont dans le dossier $APPDATA\${BUNDLEID}. Faites une sauvegarde sur la cle USB avant de le supprimer vous-meme." /SD IDOK
    ${EndIf}
  ${EndIf}
!macroend
