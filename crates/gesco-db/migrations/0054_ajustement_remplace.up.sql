-- Plan de paiement changé après un ajustement (décision du porteur du 2026-11-13) : les échéances
-- se recalculent selon le nouveau plan et l'ajustement s'applique de nouveau. L'ancien reste dans
-- l'historique de la facture, marqué remplacé à cet instant ; rien ne se supprime.
ALTER TABLE ajustement_facture ADD COLUMN remplace_le INTEGER;
