-- Licences libres (phase 10) : empreinte du poste qui installe une licence, celle à laquelle une
-- licence libre s'attache.
ALTER TABLE licence ADD COLUMN empreinte_attachee TEXT;
