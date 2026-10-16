//! Ouverture de la base, migrations, réglages de durabilité (§3.1 du blueprint, spec §8.3).
//!
//! La base est chiffrée par SQLite3 Multiple Ciphers (ChaCha20-Poly1305), compilé à la place de
//! SQLite (`vendor/libsqlite3-sys`) : décision du porteur du projet du 10 octobre 2026. La clé de
//! données de l'installation, en hexadécimal, sert de phrase de passe. Une base trouvée en clair
//! (installée avant le chiffrement) est chiffrée sur place à sa première ouverture avec clé.

use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;

use sha2::{Digest, Sha256};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{Pool, Sqlite};

use crate::erreur::{ErreurBase, Resultat};

/// Migrations embarquées dans le binaire, rejouées à l'ouverture (R-08).
pub static MIGRATIONS: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// Paramètres d'ouverture.
#[derive(Debug, Clone)]
pub struct OptionsBase {
    /// Fichier de base ; `None` ouvre une base en mémoire (tests).
    pub chemin: Option<PathBuf>,
    /// Clé de données en hexadécimal : la base est chiffrée avec. Sans clé, la base est en clair
    /// (bases en mémoire des tests, outils).
    pub cle_hex: Option<String>,
    /// Nombre maximal de connexions du pool.
    pub connexions_max: u32,
}

impl OptionsBase {
    /// Base dans un fichier.
    pub fn fichier(chemin: impl Into<PathBuf>) -> Self {
        Self {
            chemin: Some(chemin.into()),
            cle_hex: None,
            connexions_max: 4,
        }
    }

    /// Base en mémoire, une seule connexion (une base mémoire est propre à sa connexion).
    pub fn memoire() -> Self {
        Self {
            chemin: None,
            cle_hex: None,
            connexions_max: 1,
        }
    }

    /// Fournit la clé de chiffrement.
    pub fn avec_cle_hex(mut self, cle_hex: impl Into<String>) -> Self {
        self.cle_hex = Some(cle_hex.into());
        self
    }
}

/// Pools de connexions sur une base migrée.
///
/// `PRAGMA synchronous` se règle par connexion et jamais dans une transaction : la base tient donc
/// deux pools sur le même fichier, l'un en `FULL` pour les écritures critiques (financier, audit,
/// sécurité), l'autre en `NORMAL` pour le reste et les lectures (spec §8.3). En mode WAL, `NORMAL`
/// protège de la corruption ; seule la dernière transaction peut se perdre à la coupure.
#[derive(Debug, Clone)]
pub struct Base {
    pool_critique: Pool<Sqlite>,
    pool_ordinaire: Pool<Sqlite>,
    chemin: Option<PathBuf>,
}

impl Base {
    /// Ouvre, règle, migre et synchronise le catalogue des permissions.
    pub async fn ouvrir(options: OptionsBase) -> Resultat<Self> {
        let base = Self::ouvrir_sans_migrer(options).await?;
        base.migrer().await?;
        Ok(base)
    }

    /// Ouvre et règle sans migrer : le démarrage sauvegarde la base avant toute migration de
    /// schéma (R-48), puis appelle [`Base::migrer`].
    pub async fn ouvrir_sans_migrer(options: OptionsBase) -> Resultat<Self> {
        let connexion = |synchronous: SqliteSynchronous| -> Resultat<SqliteConnectOptions> {
            let mut c = match &options.chemin {
                Some(chemin) => SqliteConnectOptions::new()
                    .filename(chemin)
                    .create_if_missing(true),
                None => SqliteConnectOptions::from_str("sqlite::memory:")?,
            }
            .journal_mode(if options.chemin.is_some() {
                SqliteJournalMode::Wal
            } else {
                SqliteJournalMode::Memory
            })
            .synchronous(synchronous)
            .foreign_keys(true)
            .busy_timeout(Duration::from_secs(10));
            if let Some(cle) = &options.cle_hex {
                c = c.pragma("key", phrase_de_passe(cle)?);
            }
            Ok(c)
        };
        if let (Some(chemin), Some(cle)) = (&options.chemin, &options.cle_hex)
            && est_en_clair(chemin)?
        {
            chiffrer_sur_place(chemin, cle).await?;
        }
        let pool_critique = SqlitePoolOptions::new()
            .max_connections(options.connexions_max)
            .connect_with(connexion(SqliteSynchronous::Full)?)
            .await?;
        // Une base en mémoire est propre à sa connexion : un seul pool la dessert.
        let pool_ordinaire = if options.chemin.is_some() {
            SqlitePoolOptions::new()
                .max_connections(options.connexions_max)
                .connect_with(connexion(SqliteSynchronous::Normal)?)
                .await?
        } else {
            pool_critique.clone()
        };
        Ok(Self {
            pool_critique,
            pool_ordinaire,
            chemin: options.chemin,
        })
    }

    /// Applique les migrations en attente et synchronise le catalogue des permissions.
    pub async fn migrer(&self) -> Resultat<()> {
        MIGRATIONS.run(&self.pool_critique).await?;
        self.synchroniser_catalogue_permissions().await
    }

    /// Vrai si la base est vierge (aucune migration appliquée).
    pub async fn est_vierge(&self) -> Resultat<bool> {
        let table: Option<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name = '_sqlx_migrations'",
        )
        .fetch_optional(&self.pool_ordinaire)
        .await?;
        Ok(match table {
            None => true,
            Some(_) => self.version_schema().await? == 0,
        })
    }

    /// Version de schéma que cette version de l'application sait produire.
    pub fn version_schema_cible() -> i64 {
        MIGRATIONS.iter().map(|m| m.version).max().unwrap_or(0)
    }

    /// Vrai si des migrations restent à appliquer sur une base déjà peuplée.
    pub async fn migrations_en_attente(&self) -> Resultat<bool> {
        Ok(
            !self.est_vierge().await?
                && self.version_schema().await? < Self::version_schema_cible(),
        )
    }

    /// Pool des lectures et des écritures ordinaires (`synchronous = NORMAL`).
    pub fn pool(&self) -> &Pool<Sqlite> {
        &self.pool_ordinaire
    }

    /// Pool des écritures critiques (`synchronous = FULL`).
    pub fn pool_critique(&self) -> &Pool<Sqlite> {
        &self.pool_critique
    }

    /// Fichier de la base, s'il y en a un.
    pub fn chemin(&self) -> Option<&Path> {
        self.chemin.as_deref()
    }

    /// Dernière migration appliquée : la version de schéma portée par les sauvegardes.
    pub async fn version_schema(&self) -> Resultat<i64> {
        let table: Option<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name = '_sqlx_migrations'",
        )
        .fetch_optional(&self.pool_ordinaire)
        .await?;
        if table.is_none() {
            return Ok(0);
        }
        let version: Option<i64> =
            sqlx::query_scalar("SELECT MAX(version) FROM _sqlx_migrations WHERE success = 1")
                .fetch_one(&self.pool_ordinaire)
                .await?;
        Ok(version.unwrap_or(0))
    }

    /// `PRAGMA integrity_check`, exécuté au démarrage (§3.1).
    pub async fn verifier_integrite(&self) -> Resultat<()> {
        let resultat: String = sqlx::query_scalar("PRAGMA integrity_check")
            .fetch_one(&self.pool_ordinaire)
            .await?;
        if resultat == "ok" {
            Ok(())
        } else {
            Err(ErreurBase::Integrite(resultat))
        }
    }

    /// Force l'écriture du journal WAL dans le fichier principal, avant une copie de sauvegarde.
    pub async fn consolider_wal(&self) -> Resultat<()> {
        sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
            .execute(&self.pool_critique)
            .await?;
        Ok(())
    }

    /// Copie cohérente de la base dans un nouveau fichier (`VACUUM INTO`), sans interrompre les
    /// lecteurs : c'est l'instantané que la sauvegarde chiffre.
    pub async fn instantane(&self, destination: &Path) -> Resultat<()> {
        let chemin = destination
            .to_str()
            .ok_or_else(|| ErreurBase::Integrite("chemin d'instantané non UTF-8".to_owned()))?;
        sqlx::query("VACUUM INTO ?1")
            .bind(chemin)
            .execute(&self.pool_critique)
            .await?;
        Ok(())
    }

    /// Empreinte SHA-256 du contenu logique : chaque ligne de chaque table, valeurs citées par
    /// SQLite et triées. Deux bases de même contenu ont la même empreinte, quelle que soit la
    /// disposition des pages ; c'est ce que compare la restauration (assertion 9).
    pub async fn empreinte_contenu(&self) -> Resultat<String> {
        let mut cnx = self.pool_ordinaire.acquire().await?;
        let tables: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'
             AND name <> '_sqlx_migrations' ORDER BY name",
        )
        .fetch_all(&mut *cnx)
        .await?;
        let mut empreinte = Sha256::new();
        for table in tables {
            let colonnes: Vec<String> =
                sqlx::query_scalar("SELECT name FROM pragma_table_info(?1) ORDER BY cid")
                    .bind(&table)
                    .fetch_all(&mut *cnx)
                    .await?;
            let citees: Vec<String> = colonnes
                .iter()
                .map(|c| format!("quote(\"{}\")", c.replace('"', "\"\"")))
                .collect();
            let requete = format!(
                "SELECT {} FROM \"{}\" ORDER BY 1",
                citees.join(" || ',' || "),
                table.replace('"', "\"\"")
            );
            let mut lignes: Vec<String> = sqlx::query_scalar(sqlx::AssertSqlSafe(requete))
                .fetch_all(&mut *cnx)
                .await?;
            lignes.sort_unstable();
            empreinte.update(table.as_bytes());
            empreinte.update([0]);
            for ligne in lignes {
                empreinte.update(ligne.as_bytes());
                empreinte.update(b"\n");
            }
        }
        Ok(hex::encode(empreinte.finalize()))
    }

    /// La table `permission` est une copie du catalogue du noyau (R-02 : le noyau fait foi).
    async fn synchroniser_catalogue_permissions(&self) -> Resultat<()> {
        let mut tx = self.pool_critique.begin().await?;
        for permission in gesco_core::core::permission::CATALOGUE {
            sqlx::query("INSERT INTO permission (code, domaine) VALUES (?1, ?2) ON CONFLICT(code) DO UPDATE SET domaine = excluded.domaine")
                .bind(permission.code())
                .bind(permission.domaine())
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// Ferme proprement le pool.
    pub async fn fermer(self) {
        self.pool_ordinaire.close().await;
        self.pool_critique.close().await;
    }
}

/// En-tête d'un fichier SQLite non chiffré.
const ENTETE_EN_CLAIR: &[u8; 16] = b"SQLite format 3\0";

/// Phrase de passe du `PRAGMA key` : la clé hexadécimale, citée. Refuse tout autre caractère,
/// pour qu'aucune valeur ne sorte de la chaîne SQL.
fn phrase_de_passe(cle_hex: &str) -> Resultat<String> {
    if cle_hex.is_empty() || !cle_hex.bytes().all(|o| o.is_ascii_hexdigit()) {
        return Err(ErreurBase::Integrite(
            "clé de base non hexadécimale".to_owned(),
        ));
    }
    Ok(format!("'{cle_hex}'"))
}

/// Vrai si le fichier existe et commence par l'en-tête d'une base SQLite en clair. Un fichier
/// absent ou vide n'est pas en clair : il naîtra chiffré.
pub fn est_en_clair(chemin: &Path) -> Resultat<bool> {
    use std::io::Read;
    let mut fichier = match std::fs::File::open(chemin) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(ErreurBase::Integrite(format!("lecture de l'en-tête : {e}"))),
    };
    let mut entete = [0u8; 16];
    match fichier.read_exact(&mut entete) {
        Ok(()) => Ok(&entete == ENTETE_EN_CLAIR),
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => Ok(false),
        Err(e) => Err(ErreurBase::Integrite(format!("lecture de l'en-tête : {e}"))),
    }
}

/// Chiffre sur place une base en clair : une seule connexion sans clé, journal classique (le
/// rechiffrement est refusé en WAL ; le passage hors WAL rapatrie le journal), puis `PRAGMA rekey`
/// qui réécrit chaque page dans une transaction. Une coupure pendant l'opération laisse la base
/// en clair et intacte : la prochaine ouverture recommence.
async fn chiffrer_sur_place(chemin: &Path, cle_hex: &str) -> Resultat<()> {
    use sqlx::Connection;
    let mut cnx = sqlx::sqlite::SqliteConnection::connect_with(
        &SqliteConnectOptions::new()
            .filename(chemin)
            .journal_mode(SqliteJournalMode::Delete)
            .busy_timeout(Duration::from_secs(10)),
    )
    .await?;
    // Sûr : `phrase_de_passe` n'admet que des chiffres hexadécimaux, cités.
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "PRAGMA rekey = {}",
        phrase_de_passe(cle_hex)?
    )))
    .execute(&mut cnx)
    .await?;
    cnx.close().await?;
    if est_en_clair(chemin)? {
        return Err(ErreurBase::Integrite(
            "la base est restée en clair après son chiffrement".to_owned(),
        ));
    }
    Ok(())
}
