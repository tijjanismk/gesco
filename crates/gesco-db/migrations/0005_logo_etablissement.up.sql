-- Logo de l'établissement (SPEC_PHASE_1 §2.2), gardé dans la base : il voyage avec chaque
-- sauvegarde et sera chiffré avec elle. Une image d'1 Mo au plus ; PNG ou JPEG.
ALTER TABLE etablissement ADD COLUMN logo BLOB;
ALTER TABLE etablissement ADD COLUMN logo_format TEXT CHECK (logo_format IN ('png', 'jpeg'));
