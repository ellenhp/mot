DROP TABLE IF EXISTS poi;

CREATE TABLE poi (
  way_id BIGINT,
  node_id BIGINT,
  tags JSONB,
  geom GEOMETRY
);

SELECT UpdateGeometrySRID('poi', 'geom', 4326);

INSERT INTO poi (way_id, tags, geom) SELECT way_id, tags, geom FROM way_poi;
INSERT INTO poi (node_id, tags, geom) SELECT node_id, tags, geom FROM node_poi;

CREATE INDEX poi_geom_idx ON poi USING GIST(geom);
CREATE INDEX poi_wid_idx ON poi (way_id);
CREATE INDEX poi_nid_idx ON poi (node_id);

CREATE TABLE IF NOT EXISTS wof_admins (
  id BIGINT,
  geom GEOMETRY(MULTIPOLYGON, 4326),
  admin_level TEXT,
  names TEXT
);

CREATE INDEX IF NOT EXISTS wof_admins_geom ON wof_admins USING GIST(geom);
